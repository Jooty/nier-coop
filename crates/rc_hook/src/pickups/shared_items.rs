use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::offsets::{KEY_ITEM_FIRST_ID, KEY_ITEM_LAST_ID};
use rc_game::pickups::{
    InjectedEndWindow, ItemCountTable, PollHold, is_shared_item, partner_take_amount,
};
use rc_game::quiet_end::skipped_native_name;
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::PartnerPlace;
use rc_proto::{ItemCountEntry, ItemCounts, ItemTake, MAX_ITEM_ID, Message};

use crate::diagnostics::log_line;
use crate::net::{self, NetThread, net_clock_ms};
use crate::pickups::item_calls::{local_count, remove_for_partner};

static PARTNER_COUNTS: LazyLock<Mutex<ItemCountTable>> = LazyLock::new(Mutex::default);
static SENT_COUNTS: LazyLock<Mutex<ItemCountTable>> = LazyLock::new(Mutex::default);
static POLL_HOLD: Mutex<PollHold> = Mutex::new(PollHold::new());
static INJECTED_END_WINDOW: Mutex<InjectedEndWindow> = Mutex::new(InjectedEndWindow::new());
static PARTNER_SHARES_PHASE: AtomicBool = AtomicBool::new(false);
static PARTNER_WAS_IN_PLAY: AtomicBool = AtomicBool::new(false);

pub fn partner_count(item_id: u16) -> u8 {
    PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .count(item_id)
}

pub fn partner_shares_phase() -> bool {
    PARTNER_SHARES_PHASE.load(Ordering::Acquire)
}

pub fn lower_partner_count(item_id: u16, by: u8) {
    PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .lower(item_id, by);
}

pub fn note_take(item_id: u16) {
    let before = local_count(item_id)
        .unwrap_or(0)
        .saturating_add(partner_count(item_id));
    POLL_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_take(item_id, before, net_clock_ms());
}

pub fn frozen_count_for_poll(item_id: u16) -> Option<u8> {
    if !partner_shares_phase() || !local_is_guest() {
        return None;
    }
    POLL_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .frozen_count(item_id, net_clock_ms())
}

pub fn open_injected_end_window() {
    INJECTED_END_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open(net_clock_ms());
}

pub fn injected_end_is_running() -> bool {
    INJECTED_END_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_open(net_clock_ms())
}

pub fn skipped_in_partner_end(native_rva: u32) -> Option<&'static str> {
    let window = *INJECTED_END_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    skipped_native_name(&window, net_clock_ms(), native_rva)
}

fn local_is_guest() -> bool {
    net::handle()
        .is_some_and(|net| FollowRole::from_slot(net.view().local_slot) == FollowRole::Guest)
}

pub fn announce_local_count(item_id: u16, count: u8) {
    if !is_shared_item(item_id) || !PARTNER_WAS_IN_PLAY.load(Ordering::Acquire) {
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let changed = SENT_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .set(item_id, count);
    if changed {
        net.send_relayed(Message::ItemCounts(ItemCounts {
            replaces_all: false,
            entries: vec![ItemCountEntry { item_id, count }],
        }));
    }
}

pub fn send_take_to_partner(item_id: u16, count: u8) {
    if let Some(net) = net::handle() {
        net.send_relayed(Message::ItemTake(ItemTake { item_id, count }));
    }
}

pub fn tick() {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let in_play = view.partner_in_play();
    let shares = matches!(
        view.partner_place(),
        PartnerPlace::Same | PartnerPlace::TooFar { .. }
    );
    PARTNER_SHARES_PHASE.store(shares, Ordering::Release);
    let was_in_play = PARTNER_WAS_IN_PLAY.swap(in_play, Ordering::AcqRel);
    if in_play && !was_in_play {
        send_whole_table(net);
    }
    if !in_play && was_in_play {
        forget_partner();
    }
    let arrived = net.take_relayed(|m| matches!(m, Message::ItemCounts(_) | Message::ItemTake(_)));
    for message in arrived {
        match message {
            Message::ItemCounts(counts) => PARTNER_COUNTS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .apply(&counts),
            Message::ItemTake(take) => {
                let _ = panic::catch_unwind(|| take_for_partner(take));
            }
            _ => {}
        }
    }
}

fn send_whole_table(net: &NetThread) {
    let mut own = ItemCountTable::default();
    for item_id in 0..=MAX_ITEM_ID {
        if (KEY_ITEM_FIRST_ID..=KEY_ITEM_LAST_ID).contains(&item_id) {
            continue;
        }
        if let Some(count) = local_count(item_id) {
            own.set(item_id, count);
        }
    }
    for message in own.full_sync() {
        net.send_relayed(Message::ItemCounts(message));
    }
    *SENT_COUNTS.lock().unwrap_or_else(PoisonError::into_inner) = own;
    log_line("shared items: partner joined, sent my item counts");
}

fn forget_partner() {
    PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    SENT_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    log_line("shared items: partner left, their item counts cleared");
}

fn take_for_partner(take: ItemTake) {
    if !is_shared_item(take.item_id) {
        return;
    }
    let own = local_count(take.item_id).unwrap_or(0);
    let amount = partner_take_amount(take.count, own);
    log_line(&format!(
        "shared items: partner took {amount} of {}",
        take.item_id
    ));
    if amount > 0 {
        note_take(take.item_id);
        remove_for_partner(take.item_id, amount);
    }
}

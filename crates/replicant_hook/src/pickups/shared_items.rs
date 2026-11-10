use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::pickups::{
    InjectedEndWindow, ItemCountTable, PartnerShare, PartnerTakeQueue, PollHold,
    QueuedTakeFlushReason, is_key_item, is_shared_item, partner_take_amount,
};
use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::PartnerPlace;
use replicant_proto::{ItemCountEntry, ItemCounts, ItemTake, MAX_ITEM_ID, Message};

use crate::cutscene_skip::{end_ran_here, latest_partner_end};
use crate::diagnostics::log_line;
use crate::net::{self, NetThread, net_clock_ms};
use crate::pickups::item_calls::{local_count, remove_for_partner};

static PARTNER_COUNTS: LazyLock<Mutex<PartnerShare>> = LazyLock::new(Mutex::default);
static CAPPED_END_NAME: Mutex<String> = Mutex::new(String::new());
static SENT_COUNTS: LazyLock<Mutex<ItemCountTable>> = LazyLock::new(Mutex::default);
static POLL_HOLD: Mutex<PollHold> = Mutex::new(PollHold::new());
static PARTNER_TAKES: Mutex<PartnerTakeQueue> = Mutex::new(PartnerTakeQueue::new());
static INJECTED_END_WINDOW: Mutex<InjectedEndWindow> = Mutex::new(InjectedEndWindow::new());
static INJECTED_CALLBACK_NAME: Mutex<String> = Mutex::new(String::new());
static INJECTED_END_IS_SHARED_EVENT: AtomicBool = AtomicBool::new(false);
static PARTNER_SHARES_PHASE: AtomicBool = AtomicBool::new(false);
static PARTNER_WAS_IN_PLAY: AtomicBool = AtomicBool::new(false);

pub struct InjectedEndNow {
    pub callback: String,
    pub shared_event: bool,
}

pub fn partner_count(item_id: u16) -> u8 {
    PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .count(item_id)
}

pub fn partner_share_frozen_for() -> Option<String> {
    PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .frozen_for()
        .map(str::to_owned)
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
    if !partner_shares_phase() {
        return None;
    }
    if !local_is_guest() {
        return None;
    }
    POLL_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .frozen_count(item_id, net_clock_ms())
}

fn apply_partner_take(item_id: u16, amount: u8) {
    let before = local_count(item_id)
        .unwrap_or(0)
        .saturating_add(partner_count(item_id));
    POLL_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_take(item_id, before, net_clock_ms());
    remove_for_partner(item_id, amount);
}

fn flush_queued_partner_takes() {
    let waiting = PARTNER_TAKES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_waiting();
    if !waiting {
        return;
    }
    let end_is_over = |end_name: &str| end_ran_here(end_name) && !injected_end_is_running();
    let flushed = PARTNER_TAKES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .flush_due(net_clock_ms(), end_is_over);
    for flush in flushed {
        let item_id = flush.take.item_id;
        let amount = partner_take_amount(flush.take.amount, local_count(item_id).unwrap_or(0));
        if amount > 0 {
            apply_partner_take(item_id, amount);
        }
        let seconds = flush.waited_ms / 1000;
        let end_name = &flush.take.end_name;
        match flush.reason {
            QueuedTakeFlushReason::Cap => log_line(&format!(
                "shared items: took {amount} of {item_id} at the cap after {seconds}s ({end_name} never ran here)"
            )),
            QueuedTakeFlushReason::EndFinishedWithoutTake => log_line(&format!(
                "shared items: took {amount} of {item_id} after {seconds}s, {end_name} ran here without taking it"
            )),
        }
    }
}

pub fn take_queued_in_end(item_id: u16) -> Option<(u8, String)> {
    if !injected_end_is_running() {
        return None;
    }
    let end_name = INJECTED_CALLBACK_NAME
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    let amount = PARTNER_TAKES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_in_end(item_id, &end_name)?;
    Some((amount, end_name))
}

pub fn open_injected_end_window(callback_name: &str, shared_event: bool) {
    INJECTED_END_IS_SHARED_EVENT.store(shared_event, Ordering::Release);
    callback_name.clone_into(
        &mut INJECTED_CALLBACK_NAME
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
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

pub fn injected_end_now() -> Option<InjectedEndNow> {
    if !injected_end_is_running() {
        return None;
    }
    let callback = INJECTED_CALLBACK_NAME
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();
    Some(InjectedEndNow {
        callback,
        shared_event: INJECTED_END_IS_SHARED_EVENT.load(Ordering::Acquire),
    })
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
    flush_queued_partner_takes();
    update_partner_share_freeze();
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

fn update_partner_share_freeze() {
    let now_ms = net_clock_ms();
    let pending_end =
        latest_partner_end().filter(|(_, ran_here)| !*ran_here || injected_end_is_running());
    let mut capped = CAPPED_END_NAME
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if pending_end
        .as_ref()
        .is_none_or(|(name, _)| *name != *capped)
    {
        capped.clear();
    }
    let mut share = PARTNER_COUNTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some((end_name, _)) = pending_end else {
        release_partner_share(&mut share, "the partner's end ran here");
        return;
    };
    if end_name == *capped {
        return;
    }
    share.freeze(&end_name, now_ms);
    if share.cap_reached(now_ms) {
        end_name.clone_into(&mut capped);
        release_partner_share(&mut share, "the 60 s cap");
    }
}

fn release_partner_share(share: &mut PartnerShare, why: &str) {
    let Some(end_name) = share.frozen_for().map(str::to_owned) else {
        return;
    };
    let applied = share.release();
    log_line(&format!(
        "shared items: partner share of {end_name} released ({why}), {} lowered counts applied",
        applied.len()
    ));
}

fn send_whole_table(net: &NetThread) {
    let mut own = ItemCountTable::default();
    for item_id in 0..=MAX_ITEM_ID {
        if is_key_item(item_id) {
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
    PARTNER_TAKES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
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
    if amount == 0 {
        return;
    }
    match latest_partner_end() {
        Some((end_name, false)) => {
            log_line(&format!(
                "shared items: partner asked for {amount} of {}, waiting for {end_name} to take it here",
                take.item_id
            ));
            PARTNER_TAKES
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .queue(take.item_id, amount, &end_name, net_clock_ms());
        }
        _ => apply_partner_take(take.item_id, amount),
    }
}

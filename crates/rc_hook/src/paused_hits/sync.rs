use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rc_proto::{Message, PausedHits};

use crate::net::{NetShared, NetState, NetThread, net_clock_ms};
use crate::paused_hits::PAUSED_HITS_RESEND_MS;
use crate::paused_hits::choice::{paused_players_can_be_hit, set_paused_players_can_be_hit};

static LAST_SENT_AT_MS: AtomicU64 = AtomicU64::new(0);
static LAST_SENT_CHOICE: AtomicBool = AtomicBool::new(false);

pub fn tick(net: &NetThread, view: &NetShared) {
    let is_host = view.local_slot == Some(0);
    let connected = view.state == NetState::Connected;
    let received = net.take_relayed(|message| matches!(message, Message::PausedHits(_)));
    if !connected {
        LAST_SENT_AT_MS.store(0, Ordering::Relaxed);
        if !is_host {
            set_paused_players_can_be_hit(false);
        }
        return;
    }
    if !is_host {
        if let Some(Message::PausedHits(hits)) = received.into_iter().next_back() {
            set_paused_players_can_be_hit(hits.paused_players_can_be_hit);
        }
        return;
    }
    let now = net_clock_ms().max(1);
    let last = LAST_SENT_AT_MS.load(Ordering::Relaxed);
    let choice = paused_players_can_be_hit();
    let changed = LAST_SENT_CHOICE.swap(choice, Ordering::Relaxed) != choice;
    if changed || last == 0 || now.saturating_sub(last) >= PAUSED_HITS_RESEND_MS {
        LAST_SENT_AT_MS.store(now, Ordering::Relaxed);
        net.send_relayed(Message::PausedHits(PausedHits {
            paused_players_can_be_hit: choice,
        }));
    }
}

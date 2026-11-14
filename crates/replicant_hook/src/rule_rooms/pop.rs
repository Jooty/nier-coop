use std::sync::PoisonError;

use replicant_game::fight_sync::{PopSource, pop_source};
use replicant_game::rule_rooms::{RulePlayer, RulePop, RulePopFacts, armed_line};
use replicant_game::stay_together::FollowRole;
use replicant_game::{PhaseEntry, Replicant};
use replicant_proto::{Message, RuleRoomBreak};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetSharedExt, NetState, net_clock_ms};
use crate::rule_rooms::state::{CLOCK, WATCH};
use crate::rule_rooms::take_in::take_the_guest_in;
use crate::rule_rooms::{breaker_return, rearm};
use crate::story_relay::shared_story_events;

/// Judges one popped function against the rule rooms before the fight stream sees it: a pop it
/// `drops()` is not run; one that `skips_the_story_relay()` runs here and is never relayed.
pub(crate) fn judge_pop(
    build: &VerifiedBuild,
    manager: u64,
    name: &str,
    arguments: u64,
    queued_by_the_mod: bool,
) -> RulePop {
    let Some(net) = net::handle() else {
        return RulePop::NotARuleRoomCall;
    };
    let view = net.view();
    let Some(player) = rule_player(&view) else {
        return RulePop::NotARuleRoomCall;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    let Ok(pack) = game.current_phase() else {
        return RulePop::NotARuleRoomCall;
    };
    let local_phase = PhaseEntry::index_of(&pack).and_then(|index| u16::try_from(index).ok());
    let shared = view.state == NetState::Connected
        && shared_story_events()
        && view.partner_in_play()
        && view
            .partner_state()
            .is_some_and(|remote| Some(remote.state.phase) == local_phase);
    let facts = RulePopFacts {
        pack: &pack,
        function: name,
        player,
        shared,
        own_fire: !queued_by_the_mod && pop_source(manager, arguments) == Some(PopSource::Fired),
        queued_by_the_mod,
    };
    let pop = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .pop(&facts);
    if let Some(line) = pop.log_line(name) {
        log_line(&line);
    }
    if let RulePop::GuestTakenIn { room } = pop {
        take_the_guest_in(build, &view, room);
    }
    if let RulePop::BreakerSentBack { room, player, .. } = pop {
        breaker_return::begin(room, player);
    }
    if let RulePop::RoomRestarts { room } = pop {
        rearm::note_restart(room);
    }
    if matches!(pop, RulePop::GuestAsksTheHost { .. }) {
        send_to_the_partner(net, local_phase, name);
    }
    if matches!(pop, RulePop::Cleared { .. }) {
        CLOCK.lock().unwrap_or_else(PoisonError::into_inner).stop();
    }
    pop
}

pub(crate) fn note_registration(pack: &str, handler: &str) {
    let Some(player) = net::handle().and_then(|net| rule_player(&net.view())) else {
        return;
    };
    let armed = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .registered(pack, handler);
    if let Some(room) = armed {
        log_line(&armed_line(room, player, handler));
        if room.resets_the_room {
            CLOCK
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .start(net_clock_ms());
        }
    }
}

fn rule_player(view: &NetShared) -> Option<RulePlayer> {
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Host => Some(RulePlayer::Host),
        FollowRole::Guest => Some(RulePlayer::Guest),
        FollowRole::Unknown => None,
    }
}

pub(super) fn send_to_the_partner(net: &net::NetThread, phase: Option<u16>, handler: &str) {
    let Some(phase) = phase else {
        return;
    };
    net.send_relayed(Message::RuleRoomBreak(RuleRoomBreak {
        phase,
        handler: handler.to_owned(),
    }));
}

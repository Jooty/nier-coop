use std::sync::PoisonError;

use replicant_game::rule_rooms::{
    RoomResetVerdict, RulePlayer, guest_break_line, hosts_reset_line,
};
use replicant_game::shared_events::phase_name;
use replicant_game::stay_together::FollowRole;
use replicant_game::{PhaseEntry, Replicant};
use replicant_proto::{Message, RuleRoomBreak};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState};
use crate::rule_rooms::breaker_return;
use crate::rule_rooms::pop::send_to_the_partner;
use crate::rule_rooms::state::WATCH;
use crate::story_relay::shared_story_events;

pub(super) fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::RuleRoomBreak(_)));
    if arrived.is_empty() {
        return;
    }
    let view = net.view();
    let role = FollowRole::from_slot(view.local_slot);
    let shared = view.state == NetState::Connected && shared_story_events();
    let local_phase = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    for message in arrived {
        let Message::RuleRoomBreak(rule_break) = message else {
            continue;
        };
        if !shared || local_phase != Some(rule_break.phase) {
            log_line(&format!(
                "rule room: the partner's {} for phase {} dropped, this game is not in that phase together",
                rule_break.handler, rule_break.phase
            ));
            continue;
        }
        match role {
            FollowRole::Host => the_guest_broke(net, &rule_break),
            FollowRole::Guest => the_host_reset(&rule_break),
            FollowRole::Unknown => {}
        }
    }
}

fn the_guest_broke(net: &net::NetThread, rule_break: &RuleRoomBreak) {
    let Some(pack) = phase_name(rule_break.phase) else {
        return;
    };
    let verdict = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .accept_guest_break(pack, &rule_break.handler);
    log_line(&guest_break_line(&verdict, &rule_break.handler));
    if let RoomResetVerdict::Resets { room, .. } = verdict {
        breaker_return::begin(room, RulePlayer::Host);
        send_to_the_partner(net, Some(rule_break.phase), &rule_break.handler);
    }
}

fn the_host_reset(rule_break: &RuleRoomBreak) {
    let Some(pack) = phase_name(rule_break.phase) else {
        return;
    };
    let verdict = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .accept_hosts_reset(pack, &rule_break.handler);
    log_line(&hosts_reset_line(&verdict, &rule_break.handler));
    if let RoomResetVerdict::Resets { room, .. } = verdict {
        breaker_return::begin(room, RulePlayer::Guest);
    }
}

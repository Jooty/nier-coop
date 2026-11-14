use std::sync::PoisonError;

use replicant_game::rule_rooms::{GuestBreakVerdict, guest_break_line};
use replicant_game::shared_events::phase_name;
use replicant_game::stay_together::FollowRole;
use replicant_game::story_relay::pack_hash_of_phase;
use replicant_game::{PhaseEntry, Replicant};
use replicant_proto::{Message, RuleRoomBreak, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState};
use crate::rule_rooms::state::WATCH;
use crate::story_relay::{RelayPath, shared_story_events, submit_relay_call};

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::RuleRoomBreak(_)));
    if arrived.is_empty() {
        return;
    }
    let view = net.view();
    let is_host = view.state == NetState::Connected
        && shared_story_events()
        && FollowRole::from_slot(view.local_slot) == FollowRole::Host;
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
        if !is_host || local_phase != Some(rule_break.phase) {
            log_line(&format!(
                "rule room: the guest's {} for phase {} dropped, this game is not the host in that phase",
                rule_break.handler, rule_break.phase
            ));
            continue;
        }
        run_the_guests_break(&rule_break);
    }
}

fn run_the_guests_break(rule_break: &RuleRoomBreak) {
    let Some(pack) = phase_name(rule_break.phase) else {
        return;
    };
    let verdict = WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .accept_guest_break(pack, &rule_break.handler);
    log_line(&guest_break_line(&verdict, &rule_break.handler));
    if !matches!(verdict, GuestBreakVerdict::RunsOnce { .. }) {
        return;
    }
    let call = StoryCall {
        phase: rule_break.phase,
        pack: pack_hash_of_phase(pack),
        function: rule_break.handler.clone(),
        args: Vec::new(),
    };
    if !submit_relay_call(&call, RelayPath::HeldOwnCall) {
        // The handler never ran, so the room is still armed in the game.
        WATCH
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .registered(pack, &rule_break.handler);
        log_line(&format!(
            "rule room: the guest's {} could not be queued here, the rule stays armed",
            rule_break.handler
        ));
    }
}

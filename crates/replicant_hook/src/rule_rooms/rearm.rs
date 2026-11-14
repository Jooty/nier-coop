use std::sync::{Mutex, PoisonError};

use replicant_game::PhaseEntry;
use replicant_game::fight_sync::{SlotState, script_slot_state};
use replicant_game::rule_rooms::{REMOVE_CALLBACK_FROM_ID_NATIVE_RVA, RearmAfterRestart, RuleRoom};
use replicant_game::story_relay::pack_hash_of_phase;
use replicant_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::net::net_clock_ms;
use crate::story_jump::{NativeArg, call_native_with};
use crate::story_relay::{RelayPath, submit_relay_call};

static REARM: Mutex<RearmAfterRestart> = Mutex::new(RearmAfterRestart::new());

pub(super) fn note_restart(room: &'static RuleRoom) {
    REARM
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .restart_popped(room, net_clock_ms());
}

pub(super) fn tick(build: &VerifiedBuild) {
    let idle = script_slot_state(crate::fight_sync::script_manager_state(build)) == SlotState::Idle;
    let ready = REARM
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_if_restart_finished(net_clock_ms(), idle);
    if let Some(room) = ready {
        arm_the_breaks_again(build, room);
        log_line(&format!(
            "rule room: room {} of {}: the restart finished, the breaks and the clock are armed again",
            room.room, room.pack
        ));
    }
}

pub(super) fn arm_the_breaks_again(build: &VerifiedBuild, room: &RuleRoom) {
    for callback_id in room.removed_before_arming {
        call_native_with(
            build,
            REMOVE_CALLBACK_FROM_ID_NATIVE_RVA,
            &[NativeArg::Integer(*callback_id)],
        );
    }
    let Some(phase) = PhaseEntry::index_of(room.pack).and_then(|index| u16::try_from(index).ok())
    else {
        log_line(&format!(
            "rule room: room {} of {}: the pack is not a known phase, the breaks are not armed again",
            room.room, room.pack
        ));
        return;
    };
    for arm in room.arms {
        let call = StoryCall {
            phase,
            pack: pack_hash_of_phase(room.pack),
            function: (*arm).to_owned(),
            args: Vec::new(),
        };
        if !submit_relay_call(&call, RelayPath::HeldOwnCall) {
            log_line(&format!(
                "rule room: room {} of {}: {arm} could not be queued here, the rule stays unarmed",
                room.room, room.pack
            ));
        }
    }
}

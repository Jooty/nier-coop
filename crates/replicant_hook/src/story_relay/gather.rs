use std::sync::{Mutex, PoisonError};

use replicant_game::stay_together::local_phase_index;
use replicant_game::story_calls::PartnerPlace;
use replicant_game::story_relay::{
    GatherAdmission, GatherAloneReason, GatherInputs, GatherOutcome, GatherRelease,
    SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS,
};
use replicant_game::{GameMemory, Replicant};
use replicant_proto::{GatherForScene, Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{NetShared, NetSharedExt, NetState, NetThread, net_clock_ms};
use crate::stay_together::side_view_now;
use crate::story_relay::call_gate::{RelayPath, relay_queue_has_room, submit_relay_call};
use crate::story_relay::state::GATHER_HOLD;

static RELEASED_WAITING_FOR_THE_QUEUE: Mutex<Vec<GatherRelease>> = Mutex::new(Vec::new());

pub(super) fn admission_is_open(phase: u16, function: &str) -> bool {
    GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(phase, function, net_clock_ms())
        == GatherAdmission::Open
}

pub(super) fn host_may_gather(build: &VerifiedBuild) -> bool {
    !side_view_now(build)
}

pub(super) fn hold_scene_call<M: GameMemory>(
    net: &NetThread,
    game: &Replicant<M>,
    call: StoryCall,
    partner_place: PartnerPlace,
) -> bool {
    if !relay_queue_has_room() {
        return false;
    }
    let Ok(position) = game.position() else {
        return false;
    };
    let function = call.function.clone();
    let phase = call.phase;
    let held = GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .hold(call, net_clock_ms());
    if !held {
        return false;
    }
    net.send_relayed(Message::GatherForScene(GatherForScene {
        phase,
        position: [position.x, position.y, position.z],
    }));
    log_line(&format!(
        "story event: {function} waits up to {SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS} ms, the partner is {} and is asked to come",
        partner_place.describe()
    ));
    true
}

pub(super) fn tick(build: &VerifiedBuild, view: Option<&NetShared>) {
    retry_releases_the_queue_refused();
    let idle = GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_idle();
    if idle {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let inputs = GatherInputs {
        now_ms: net_clock_ms(),
        connected: view.is_some_and(|view| view.state == NetState::Connected),
        partner_in_play: view.is_some_and(NetShared::partner_in_play),
        partner_place: view.map_or(PartnerPlace::Unknown, NetShared::partner_place),
        local_phase: local_phase_index(&game),
    };
    let step = GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(&inputs);
    for call in step.forgotten {
        log_line(&format!(
            "story event: {} was waiting for the partner and is dropped, the phase changed",
            call.function
        ));
    }
    for release in step.released {
        start_released(release);
    }
}

fn start_released(release: GatherRelease) {
    match release.outcome {
        GatherOutcome::PartnerArrived => log_line(&format!(
            "story event: {} starts now, the partner arrived after {} ms",
            release.call.function, release.waited_ms
        )),
        GatherOutcome::RunAlone(reason) => log_line(&format!(
            "story event: {} starts here only, the partner did not arrive ({})",
            release.call.function,
            alone_reason_text(reason)
        )),
    }
    if !submit_relay_call(&release.call, RelayPath::HeldHostCall) {
        RELEASED_WAITING_FOR_THE_QUEUE
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(release);
    }
}

fn retry_releases_the_queue_refused() {
    let waiting = std::mem::take(
        &mut *RELEASED_WAITING_FOR_THE_QUEUE
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    for release in waiting {
        if !submit_relay_call(&release.call, RelayPath::HeldHostCall) {
            RELEASED_WAITING_FOR_THE_QUEUE
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(release);
        }
    }
}

fn alone_reason_text(reason: GatherAloneReason) -> &'static str {
    match reason {
        GatherAloneReason::TimeLimit => "time limit reached",
        GatherAloneReason::PartnerNotInPlay => "the partner is not in play",
        GatherAloneReason::ConnectionLost => "connection lost",
        GatherAloneReason::PartnerInAnotherPhase => "the partner is in another area",
    }
}

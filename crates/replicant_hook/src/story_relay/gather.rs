use std::sync::{Mutex, PoisonError};

use replicant_game::callback_gate::{ScenePull, ScenePullFacts, scene_pull};
use replicant_game::script_functions::script_function;
use replicant_game::shared_events::phase_name;
use replicant_game::stay_together::local_phase_index;
use replicant_game::story_calls::{PartnerPlace, is_event_callback_name};
use replicant_game::story_relay::{
    GatherAdmission, GatherAloneReason, GatherInputs, GatherOutcome, GatherRelease,
    SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS, describe_partner_reply,
};
use replicant_game::{GameMemory, Replicant};
use replicant_proto::{GatherForScene, Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::own_side_view_door;
use crate::memory::InProcessMemory;
use crate::net::{NetShared, NetSharedExt, NetState, NetThread, net_clock_ms};
use crate::stay_together::side_view_now;
use crate::story_relay::call_gate::{RelayPath, relay_queue_has_room, submit_relay_call};
use crate::story_relay::state::{GATHER_HOLD, HOOKED_BUILD};

static RELEASED_WAITING_FOR_THE_QUEUE: Mutex<Vec<GatherRelease>> = Mutex::new(Vec::new());

pub(super) fn admission_is_open(phase: u16, function: &str) -> bool {
    admission(phase, function) == GatherAdmission::Open
}

fn admission(phase: u16, function: &str) -> GatherAdmission {
    GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(phase, function, net_clock_ms())
}

/// True when the player's own scene start was taken out of the game's queue: it waits for the
/// partner to come over and is put back by the tick, or it repeats a start that already waits.
pub(crate) fn own_scene_start_waits_for_the_partner(
    net: &NetThread,
    phase: u16,
    name: &str,
    partner_place: PartnerPlace,
    led_by_partner: bool,
) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let starts_scene = is_event_callback_name(name)
        && phase_name(phase)
            .and_then(|label| script_function(label, name))
            .is_some_and(|function| function.starts_scene);
    let may_wait = starts_scene && !led_by_partner;
    let facts = ScenePullFacts {
        starts_scene,
        led_by_partner,
        partner_place,
        // Asking consumes the mark a released start leaves, so only a start that may wait asks.
        admission: if may_wait {
            admission(phase, name)
        } else {
            GatherAdmission::Open
        },
        relay_queue_has_room: relay_queue_has_room(),
    };
    match scene_pull(&facts) {
        ScenePull::WaitAndAskPartnerOver { .. } => {
            let call = StoryCall {
                phase,
                pack: 0,
                function: name.to_owned(),
                args: Vec::new(),
            };
            let game = Replicant::new(InProcessMemory::new(build));
            hold_scene_call(build, net, &game, call, partner_place)
        }
        ScenePull::DropRepeatedFire => {
            log_line(&format!(
                "story event: {name} fired again while it waits for the partner, this fire is dropped"
            ));
            true
        }
        ScenePull::RunNow(skip) => {
            if skip.is_worth_a_log_line() {
                log_line(&format!(
                    "story event: {name} does not wait for the partner ({})",
                    skip.describe()
                ));
            }
            false
        }
    }
}

pub(super) fn hold_scene_call<M: GameMemory>(
    build: &VerifiedBuild,
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
        in_side_view: side_view_now(build),
        side_view_door: own_side_view_door(),
    }));
    log_line(&format!(
        "story event: {function} waits up to {SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS} ms, the partner is {} and is asked to come",
        partner_place.describe()
    ));
    true
}

pub(super) fn tick(build: &VerifiedBuild, net: Option<&NetThread>) {
    retry_releases_the_queue_refused();
    let answers = net.map_or_else(Vec::new, |net| {
        net.take_relayed(|message| matches!(message, Message::GatherAnswer(_)))
    });
    let idle = GATHER_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_idle();
    if idle {
        return;
    }
    for message in answers {
        let Message::GatherAnswer(answer) = message else {
            continue;
        };
        GATHER_HOLD
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .partner_answered(answer.phase, answer.reply);
    }
    let view = net.map(|net| net.view());
    let view = view.as_ref();
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
    if !submit_relay_call(&release.call, RelayPath::HeldOwnCall) {
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
        if !submit_relay_call(&release.call, RelayPath::HeldOwnCall) {
            RELEASED_WAITING_FOR_THE_QUEUE
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(release);
        }
    }
}

fn alone_reason_text(reason: GatherAloneReason) -> &'static str {
    match reason {
        GatherAloneReason::PartnerStays(reply) => describe_partner_reply(reply),
        GatherAloneReason::TimeLimit => "time limit reached",
        GatherAloneReason::PartnerNotInPlay => "the partner is not in play",
        GatherAloneReason::ConnectionLost => "connection lost",
        GatherAloneReason::PartnerInAnotherPhase => "the partner is in another area",
    }
}

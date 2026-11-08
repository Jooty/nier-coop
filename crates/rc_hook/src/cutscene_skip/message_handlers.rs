use std::panic;
use std::sync::{Mutex, PoisonError};

use rc_game::callback_gate::{GateRole, ParkedCall, ParkedKind, is_end_callback};
use rc_game::message_handlers::{
    HandlerArrival, HandlerArrivalFacts, HandlerFire, HandlerFireFacts, RelayedHandlerPasses,
    handler_arrival, handler_fire,
};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::shared_events::{is_shared_event, phase_name};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, MessageHandlerRan, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::callback_gate::{WAITING_ENDS, gate_role, note_ran_end, shared_phase};
use crate::cutscene_skip::shared_functions::{
    close_menus_for_modal_entry, forced_drain, note_run_from_partner,
};
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::story_relay::inject::queue_call;
use crate::story_relay::{is_host_driven_handler, shared_story_events};

static PASSES: Mutex<RelayedHandlerPasses> = Mutex::new(RelayedHandlerPasses::new());

pub(super) fn reset() {
    PASSES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
}

pub(super) fn judge_popped(name: &str) -> HandlerFire {
    panic::catch_unwind(|| judge(name)).unwrap_or(HandlerFire::NotAHandler)
}

fn judge(name: &str) -> HandlerFire {
    let Some(net) = net::handle() else {
        return HandlerFire::NotAHandler;
    };
    let view = net.view();
    let Some(local) = view.local.as_ref().map(|local| local.phase) else {
        return HandlerFire::NotAHandler;
    };
    let facts = HandlerFireFacts {
        host_driven_handler: is_host_driven_handler(local, name),
        role: gate_role(&view),
        both_in_phase: shared_phase(&view) == Some(local),
    };
    let fire = handler_fire(
        &facts,
        &mut PASSES.lock().unwrap_or_else(PoisonError::into_inner),
        name,
        net_clock_ms(),
    );
    match fire {
        HandlerFire::NotAHandler => {}
        HandlerFire::RunHereOnly => log_line(&format!(
            "message handler: {name} runs here only, the partner is not in this phase"
        )),
        HandlerFire::HostRunsAndSends => {
            if is_end_callback(name) {
                note_ran_end(name, local);
            }
            net.send_relayed(Message::MessageHandlerRan(MessageHandlerRan {
                phase: local,
                name: name.to_owned(),
            }));
            log_line(&format!(
                "message handler: {name} ran here, sent to the guest"
            ));
        }
        HandlerFire::GuestRunsTheHostsCall => {
            log_line(&format!(
                "message handler: {name} runs here, it is the host's call"
            ));
        }
        HandlerFire::GuestHoldsForTheHost => {
            log_line(&format!("message handler: {name} held for the host"))
        }
    }
    fire
}

pub(super) fn receive_message_handler(
    build: &VerifiedBuild,
    role: GateRole,
    ran: &MessageHandlerRan,
) {
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    let label = phase_name(ran.phase).unwrap_or_default();
    let queue_is_empty = WAITING_ENDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_empty();
    let facts = HandlerArrivalFacts {
        enabled: shared_story_events(),
        local_is_guest: role == GateRole::Guest,
        local_in_play: classify(read_play_checks(&game)).is_in_play(),
        local_phase,
        call_phase: ran.phase,
        phase_label: label,
        queue_is_empty,
    };
    let call = StoryCall {
        phase: ran.phase,
        pack: 0,
        function: ran.name.clone(),
        args: Vec::new(),
    };
    match handler_arrival(&facts, &ran.name) {
        HandlerArrival::Drop(reason) => log_line(&format!(
            "message handler: {} from the partner dropped ({})",
            ran.name,
            reason.describe()
        )),
        HandlerArrival::RunNow => {
            log_line(&format!(
                "message handler: {} from the host runs now (nothing waiting ahead of it)",
                ran.name
            ));
            run_message_handler(build, &call);
        }
        HandlerArrival::Park => {
            let parked = ParkedCall {
                kind: ParkedKind::Handler,
                call,
            };
            let position = WAITING_ENDS
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .park_call_always(parked, net_clock_ms());
            log_line(&format!(
                "message handler: {} from the host queued at position {position:?} behind earlier partner calls",
                ran.name
            ));
        }
        HandlerArrival::ForcedDrain => {
            forced_drain(build, "host", label, call, ParkedKind::Handler)
        }
    }
}

pub(crate) fn run_message_handler(build: &VerifiedBuild, call: &StoryCall) {
    let name = call.function.as_str();
    let now = net_clock_ms();
    close_menus_for_modal_entry(build, call.phase, name);
    note_run_from_partner(call.phase, name, now);
    if is_end_callback(name) {
        let shared_event = phase_name(call.phase).is_some_and(|label| is_shared_event(label, name));
        crate::pickups::open_injected_end_window(name, shared_event);
        note_ran_end(name, call.phase);
    }
    PASSES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .expect_run(name, now);
    let queued = queue_call(build, call).is_some();
    log_line(&format!(
        "message handler: {name} runs here from the host's call (queued {queued})"
    ));
}

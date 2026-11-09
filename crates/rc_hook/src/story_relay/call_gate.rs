use std::sync::{Mutex, PoisonError};

use rc_game::fight_sync::script_slot_state;
use rc_game::script_functions::shows_modal_ui;
use rc_game::shared_events::phase_name;
use rc_game::story_relay::{
    CallKey, MAX_WAITING_RELAY_CALLS, QueueRetry, RelayCallGate, RelayCallNotice,
    RelayCallSubmitted, WaitingRelayCall,
};
use rc_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::{close_menus_for_modal_entry, expect_handler_run, forget_handler_run};
use crate::diagnostics::log_line;
use crate::net::net_clock_ms;
use crate::story_relay::inject::queue_call_now;
use crate::story_relay::state::LEDGER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelayPath {
    CallbackFollow,
    MessageHandler,
    SharedFunction,
    SceneFollow,
    StoryEvent(CallKey),
}

static RELAY_CALLS: Mutex<RelayCallGate<RelayPath>> = Mutex::new(RelayCallGate::new());

pub(crate) fn submit_relay_call(call: &StoryCall, path: RelayPath) -> bool {
    let submitted = RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .submit(call.clone(), path, net_clock_ms());
    match submitted {
        RelayCallSubmitted::Waiting { position } => {
            if position > 1 {
                log_line(&format!(
                    "relay calls: {} waits at position {position}, one partner call goes into your game's queue at a time",
                    call.function
                ));
            }
            true
        }
        RelayCallSubmitted::Full => {
            log_line(&format!(
                "relay calls: {} not queued, {MAX_WAITING_RELAY_CALLS} partner calls wait already",
                call.function
            ));
            false
        }
    }
}

pub fn tick_relay_calls(build: &VerifiedBuild) {
    let slot = script_slot_state(crate::fight_sync::script_manager_state(build));
    let now = net_clock_ms();
    let fight_call_queued = crate::fight_sync::own_call_queued();
    let due = {
        let mut gate = RELAY_CALLS.lock().unwrap_or_else(PoisonError::into_inner);
        gate.check_queued(slot, now, |name, arguments| {
            crate::fight_sync::queued_call_is_ours(build, name, arguments)
        });
        gate.tick(slot, fight_call_queued, |call| {
            shows_modal_ui(phase_name(call.phase).unwrap_or_default(), &call.function)
        })
    };
    if let Some(due) = due {
        queue_now(build, due, now);
    }
    let notices = RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_notices();
    for notice in notices {
        log_line(&match notice {
            RelayCallNotice::QueuedCallNeverPopped { name, waited_ms } => format!(
                "relay calls: the partner's {name} was queued here but did not pop within {waited_ms} ms and left the queue, the next call may go"
            ),
            RelayCallNotice::QueuedCallStillWaiting { name, waited_ms } => format!(
                "relay calls: the partner's {name} is still in your game's queue after {waited_ms} ms, the next call waits for it"
            ),
            RelayCallNotice::QueueingGaveUp { name, tries } => format!(
                "relay calls: your game's queue refused the partner's {name} {tries} times, dropped"
            ),
        });
    }
}

fn queue_now(build: &VerifiedBuild, due: WaitingRelayCall<RelayPath>, now: u64) {
    let call = &due.call;
    let name = call.function.as_str();
    close_menus_for_modal_entry(build, call.phase, name);
    if due.then == RelayPath::MessageHandler {
        expect_handler_run(name, now);
    }
    let Some(slot) = queue_call_now(build, call) else {
        if due.then == RelayPath::MessageHandler {
            forget_handler_run(name, now);
        }
        let path = due.then;
        let name = name.to_owned();
        let retry = RELAY_CALLS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .queue_failed(due);
        if retry == (QueueRetry::Again { tries: 1 }) {
            log_line(&format!(
                "relay calls: could not queue the partner's {name} ({path:?}), your game's queue is full, tried again each tick"
            ));
        }
        return;
    };
    RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .queued(name, slot);
    if let RelayPath::StoryEvent(key) = due.then {
        LEDGER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_injected(slot, key, now);
    }
    log_line(&format!(
        "relay calls: the partner's {name} queued here ({:?}, waited {} ms)",
        due.then,
        now.saturating_sub(due.submitted_ms)
    ));
}

pub(crate) fn relay_call_popped(arguments: u64) {
    RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .popped(arguments);
}

pub(super) fn relay_call_cancelled(arguments: u64) {
    RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .cancelled(arguments);
}

pub(crate) fn relay_calls_are_clear() -> bool {
    RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_clear()
}

pub(crate) fn relay_calls_enter_phase(phase: Option<u16>) {
    let dropped = RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .enter_phase(phase);
    if dropped > 0 {
        log_line(&format!(
            "relay calls: {dropped} waiting partner call(s) dropped, you are in another phase now"
        ));
    }
}

pub(crate) fn forget_relay_call_in_flight(why: &str) {
    let forgotten = RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget_in_flight();
    if let Some(name) = forgotten {
        log_line(&format!(
            "relay calls: the partner's {name} is no longer tracked in your game's queue ({why})"
        ));
    }
}

pub(crate) fn drop_relay_calls(why: &str) {
    let dropped = RELAY_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    if dropped > 0 {
        log_line(&format!(
            "relay calls: {dropped} waiting partner call(s) dropped ({why})"
        ));
    }
}

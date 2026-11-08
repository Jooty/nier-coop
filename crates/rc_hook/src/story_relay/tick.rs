use std::sync::PoisonError;

use rc_game::callback_gate::ParkedKind;
use rc_game::readiness::{classify, read_play_checks};
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::{SceneScope, StoryCallKind, is_event_callback_name, name_hash};
use rc_game::story_relay::{
    CallKey, HeldTimeoutVerdict, RefusedAction, RunAction, RunFacts, TriggerAction, args_hash,
    held_timeout_verdict, relay_kind,
};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, StoryCall, StoryRefusalReason, StoryRefused};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, NetThread, net_clock_ms};
use crate::story_relay::dummy_hits;
use crate::story_relay::inject::queue_call;
use crate::story_relay::parked::deliver;
use crate::story_relay::settings::shared_story_events;
use crate::story_relay::state::{HELD_CALLS, HOOKED_BUILD, LEDGER};

enum Arrived {
    Trigger,
    Run,
    Refused(StoryRefusalReason),
}

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_ok() {
        dummy_hits::install(build);
        log_line("story relay: ready, chained into the zone gate's callback pop hook");
    }
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| {
        matches!(
            message,
            Message::StoryTrigger(_)
                | Message::StoryRun(_)
                | Message::StoryRefused(_)
                | Message::DummyHit(_)
        )
    });
    let view = net.view();
    if view.state != NetState::Connected {
        drop_session_state(build);
        return;
    }
    let timed_out = {
        let mut ledger = LEDGER.lock().unwrap_or_else(PoisonError::into_inner);
        ledger.begin_tick();
        ledger.take_timed_out_held(net_clock_ms())
    };
    if arrived.is_empty() && timed_out.is_empty() {
        return;
    }
    let role = FollowRole::from_slot(view.local_slot);
    let partner_place = view.partner_place();
    let game = Replicant::new(InProcessMemory::new(build));
    let local_in_play = classify(read_play_checks(&game)).is_in_play();
    let local_phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    for key in timed_out {
        handle_timed_out(build, key, local_in_play, local_phase);
    }
    for message in arrived {
        if let Message::DummyHit(call) = &message {
            dummy_hits::on_arrival(build, call, local_in_play, local_phase);
            continue;
        }
        let (call, kind) = match message {
            Message::StoryTrigger(call) => (call, Arrived::Trigger),
            Message::StoryRun(call) => (call, Arrived::Run),
            Message::StoryRefused(refused) => (refused.call, Arrived::Refused(refused.reason)),
            _ => continue,
        };
        let function_hash = name_hash(&call.function);
        let key = CallKey {
            function: function_hash,
            args: args_hash(&call.args),
        };
        let facts = RunFacts {
            enabled: shared_story_events(),
            role,
            local_in_play,
            local_phase,
            call_phase: call.phase,
            known_story_call: relay_kind(call.pack, function_hash)
                .is_some_and(StoryCallKind::is_relayed)
                && !is_event_callback_name(&call.function),
            scope: SceneScope::of(&call.function),
            partner_here: partner_place.is_same(),
        };
        let ids = format!("fn={function_hash:016x} args={:016x}", key.args);
        match kind {
            Arrived::Trigger => handle_trigger(build, net, &call, &facts, key, &ids),
            Arrived::Run => handle_run(build, &call, &facts, key, &ids),
            Arrived::Refused(reason) => {
                handle_refused(build, key, reason, local_in_play, local_phase, &ids);
            }
        }
    }
}

fn handle_timed_out(
    build: &VerifiedBuild,
    key: CallKey,
    local_in_play: bool,
    local_phase: Option<u16>,
) {
    let call_phase = HELD_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(held_key, _)| *held_key == key)
        .map(|(_, call)| call.phase);
    let Some(call_phase) = call_phase else {
        return;
    };
    match held_timeout_verdict(local_in_play, local_phase, call_phase) {
        HeldTimeoutVerdict::RunHere => {
            if run_held_here(build, key, local_in_play, local_phase, "no answer in time") {
                LEDGER
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .note_ran_locally(key, net_clock_ms());
            }
        }
        HeldTimeoutVerdict::KeepWaiting => {
            LEDGER
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .hold_call(key, net_clock_ms());
        }
        HeldTimeoutVerdict::Drop => {
            take_held_call(key);
            log_line(&format!(
                "story event: guest's held call dropped, the phase changed fn={:016x} args={:016x}",
                key.function, key.args
            ));
        }
    }
}

fn drop_session_state(build: &VerifiedBuild) {
    let held_keys: Vec<CallKey> = HELD_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .map(|(key, _)| *key)
        .collect();
    if held_keys.is_empty()
        && LEDGER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_idle()
    {
        return;
    }
    if !held_keys.is_empty() {
        let game = Replicant::new(InProcessMemory::new(build));
        let local_in_play = classify(read_play_checks(&game)).is_in_play();
        let local_phase = game
            .current_phase()
            .ok()
            .as_deref()
            .and_then(PhaseEntry::index_of)
            .and_then(|index| u16::try_from(index).ok());
        for key in held_keys {
            run_held_here(build, key, local_in_play, local_phase, "the session ended");
        }
    }
    HELD_CALLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear_session();
}

fn handle_refused(
    build: &VerifiedBuild,
    key: CallKey,
    reason: StoryRefusalReason,
    local_in_play: bool,
    local_phase: Option<u16>,
    ids: &str,
) {
    let action = LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_refused(key, reason, net_clock_ms());
    match action {
        RefusedAction::NotHeld => {
            log_line(&format!("story event: refusal for a call not held {ids}"));
        }
        RefusedAction::IgnoreLate => {
            log_line(&format!(
                "story event: late refusal ignored, the guest already ran its held call {ids}"
            ));
        }
        RefusedAction::ForgetOnly => {
            take_held_call(key);
            log_line(&format!(
                "story event: host already ran {ids}, guest's held call forgotten"
            ));
        }
        RefusedAction::RunHere => {
            run_held_here(
                build,
                key,
                local_in_play,
                local_phase,
                &format!("host refused: {reason:?}"),
            );
        }
    }
}

fn take_held_call(key: CallKey) -> Option<StoryCall> {
    let mut held = HELD_CALLS.lock().unwrap_or_else(PoisonError::into_inner);
    let position = held.iter().position(|(held_key, _)| *held_key == key)?;
    Some(held.remove(position).1)
}

fn run_held_here(
    build: &VerifiedBuild,
    key: CallKey,
    local_in_play: bool,
    local_phase: Option<u16>,
    why: &str,
) -> bool {
    let Some(call) = take_held_call(key) else {
        return false;
    };
    let ids = format!("fn={:016x} args={:016x}", key.function, key.args);
    if !local_in_play || local_phase != Some(call.phase) {
        log_line(&format!(
            "story event: guest's held call not run, no longer in the same place {ids} ({why})"
        ));
        return false;
    }
    let queued = inject_and_mark(build, &call, key, &ids);
    if queued {
        log_line(&format!(
            "story event: guest runs its held call here {ids} ({why})"
        ));
    }
    queued
}

fn handle_trigger(
    build: &VerifiedBuild,
    net: &NetThread,
    call: &StoryCall,
    facts: &RunFacts,
    key: CallKey,
    ids: &str,
) {
    let action = LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_trigger(facts, key, net_clock_ms());
    match action {
        TriggerAction::Drop(reason) => {
            net.send_relayed(Message::StoryRefused(StoryRefused {
                call: call.clone(),
                reason: reason.into(),
            }));
            log_line(&format!(
                "story event: trigger of {} dropped {ids} ({reason:?}), refusal sent",
                call.function
            ));
        }
        TriggerAction::InjectAndBroadcast => {
            deliver(build, ParkedKind::AcceptedTrigger, call);
        }
    }
}

fn handle_run(build: &VerifiedBuild, call: &StoryCall, facts: &RunFacts, key: CallKey, ids: &str) {
    let action = LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_run(facts, key, net_clock_ms());
    take_held_call(key);
    match action {
        RunAction::IgnoreLate => {
            log_line(&format!(
                "story event: late run ignored, the guest already ran its held call {ids}"
            ));
        }
        RunAction::Drop(reason) => {
            log_line(&format!(
                "story event: run of {} dropped {ids} ({reason:?})",
                call.function
            ));
        }
        RunAction::Inject => deliver(build, ParkedKind::Run, call),
    }
}

pub(super) fn inject_and_mark(
    build: &VerifiedBuild,
    call: &StoryCall,
    key: CallKey,
    ids: &str,
) -> bool {
    let Some(arguments_pointer) = queue_call(build, call) else {
        log_line(&format!("story event: could not queue {ids} (queue full)"));
        return false;
    };
    LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_injected(arguments_pointer, key, net_clock_ms());
    true
}

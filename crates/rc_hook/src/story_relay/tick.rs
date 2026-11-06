use std::sync::PoisonError;

use rc_game::readiness::{classify, read_play_checks};
use rc_game::stay_together::FollowRole;
use rc_game::story_calls::{StoryCallKind, name_hash};
use rc_game::story_relay::{CallKey, RunAction, RunFacts, TriggerAction, args_hash, relay_kind};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState, NetThread, net_clock_ms};
use crate::story_relay::inject::queue_call;
use crate::story_relay::settings::shared_story_events;
use crate::story_relay::state::{HOOKED_BUILD, LEDGER};

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_ok() {
        log_line("story relay: ready, chained into the zone gate's callback pop hook");
    }
}

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net
        .take_relayed(|message| matches!(message, Message::StoryTrigger(_) | Message::StoryRun(_)));
    if arrived.is_empty() {
        return;
    }
    let view = net.view();
    if view.state != NetState::Connected {
        return;
    }
    let role = FollowRole::from_slot(view.local_slot);
    let game = Replicant::new(InProcessMemory::new(build));
    let local_in_play = classify(read_play_checks(&game)).is_in_play();
    let local_phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    for message in arrived {
        let (call, is_trigger) = match message {
            Message::StoryTrigger(call) => (call, true),
            Message::StoryRun(call) => (call, false),
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
                .is_some_and(StoryCallKind::is_relayed),
        };
        let ids = format!("fn={function_hash:016x} args={:016x}", key.args);
        if is_trigger {
            handle_trigger(build, net, &call, &facts, key, &ids);
        } else {
            handle_run(build, &call, &facts, key, &ids);
        }
    }
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
            log_line(&format!("story event: trigger dropped {ids} ({reason:?})"));
        }
        TriggerAction::InjectAndBroadcast => {
            if inject_and_mark(build, call, ids) {
                net.send_relayed(Message::StoryRun(call.clone()));
                log_line(&format!("story event: host accepted {ids}, run sent"));
            }
        }
    }
}

fn handle_run(build: &VerifiedBuild, call: &StoryCall, facts: &RunFacts, key: CallKey, ids: &str) {
    let action = LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_run(facts, key, net_clock_ms());
    match action {
        RunAction::Drop(reason) => {
            log_line(&format!("story event: run dropped {ids} ({reason:?})"));
        }
        RunAction::Inject => {
            if inject_and_mark(build, call, ids) {
                log_line(&format!("story event: guest queued the host's run {ids}"));
            }
        }
    }
}

fn inject_and_mark(build: &VerifiedBuild, call: &StoryCall, ids: &str) -> bool {
    let Some(arguments_pointer) = queue_call(build, call) else {
        log_line(&format!("story event: could not queue {ids} (queue full)"));
        return false;
    };
    LEDGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_injected(arguments_pointer, net_clock_ms());
    true
}

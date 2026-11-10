use rc_game::callback_gate::{ParkedCall, ParkedKind};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::story_calls::name_hash;
use rc_game::story_relay::{CallKey, args_hash};
use rc_game::{PhaseEntry, Replicant};
use rc_proto::{Message, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::park_partner_call;
use crate::cutscene_skip::{run_message_handler, run_shared_function};
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;
use crate::story_relay::tick::inject_and_mark;

pub(super) fn deliver(build: &VerifiedBuild, kind: ParkedKind, call: &StoryCall) {
    let parked = ParkedCall {
        kind,
        call: call.clone(),
    };
    if park_partner_call(build, parked) {
        return;
    }
    release(build, kind, call);
}

pub(crate) fn run_parked(
    build: &VerifiedBuild,
    parked: &ParkedCall,
    position: u32,
    waited_ms: u64,
) {
    let call = &parked.call;
    let game = Replicant::new(InProcessMemory::new(build));
    let local_in_play = classify(read_play_checks(&game)).is_in_play();
    let local_phase = game
        .current_phase()
        .ok()
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    if !local_in_play || local_phase != Some(call.phase) {
        log_line(&format!(
            "partner calls: {} (queue position {position}) not run, no longer in the same place",
            call.function
        ));
        return;
    }
    log_line(&format!(
        "partner calls: {} (queue position {position}) runs now after waiting {} ms",
        call.function, waited_ms
    ));
    release(build, parked.kind, call);
}

fn release(build: &VerifiedBuild, kind: ParkedKind, call: &StoryCall) {
    let key = CallKey {
        function: name_hash(&call.function),
        args: args_hash(&call.args),
    };
    let ids = format!("fn={:016x} args={:016x}", key.function, key.args);
    match kind {
        ParkedKind::Function => run_shared_function(build, call),
        ParkedKind::Handler => run_message_handler(build, call),
        ParkedKind::Run => {
            if inject_and_mark(call, key, &ids) {
                log_line(&format!("story event: guest queued the host's run {ids}"));
            }
        }
        ParkedKind::AcceptedTrigger => {
            if inject_and_mark(call, key, &ids)
                && let Some(net) = net::handle()
            {
                net.send_relayed(Message::StoryRun(call.clone()));
                log_line(&format!("story event: host accepted {ids}, run sent"));
            }
        }
    }
}

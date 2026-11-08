use std::sync::{Mutex, PoisonError};

use rc_game::move_gate::{
    MoveAsk, MoveHolds, MoveRelease, MoveVerdict, StartAnswer, judge_move, moves_the_player,
};
use rc_game::zone_rules::{Companion, Placement};
use rc_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::callback_gate::note_talk_command;
use crate::cutscene_skip::scene_follow::local_phase;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::net::{self, NetShared, NetState, net_clock_ms};
use crate::story_relay::inject::queue_call;
use crate::zone_rules::zone_radius;

static HOLDS: Mutex<MoveHolds> = Mutex::new(MoveHolds::new());

pub(super) fn holds_start(name: &str, after_talk_command: bool) -> bool {
    if !moves_the_player(name) {
        return false;
    }
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let Some(phase) = view.local.as_ref().map(|local| local.phase) else {
        return false;
    };
    let verdict = verdict_for(&view);
    let answer = HOLDS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_start(name, phase, after_talk_command, &verdict, net_clock_ms());
    let StartAnswer::Hold { first } = answer else {
        return false;
    };
    if first && let MoveVerdict::Wait { distance_units, .. } = &verdict {
        log_line(&format!(
            "move gate: {name} moves you, waiting for the partner ({distance_units:.0} m away)"
        ));
        post_event(&verdict.prompt_text().unwrap_or_default());
    }
    true
}

pub(super) fn tick(build: &VerifiedBuild, view: &NetShared) {
    let verdict = verdict_for(view);
    let now = net_clock_ms();
    let (released, remind) = {
        let mut holds = HOLDS.lock().unwrap_or_else(PoisonError::into_inner);
        if !holds.is_holding() {
            return;
        }
        let released = holds.step(&verdict, local_phase(build), now);
        (released, holds.reminder_due(now))
    };
    if remind && let Some(text) = verdict.prompt_text() {
        post_event(&text);
    }
    for item in released {
        let name = item.held.name;
        match item.why {
            MoveRelease::LeftPhase => {
                log_line(&format!(
                    "move gate: you left the area, {name} is no longer waiting"
                ));
                continue;
            }
            MoveRelease::PartnerArrived => log_line(&format!(
                "move gate: the partner is there (or gone), {name} runs now"
            )),
            MoveRelease::GaveUp { waited_ms } => log_line(&format!(
                "move gate: gave up waiting for the partner after {waited_ms} ms, {name} runs now"
            )),
        }
        if item.held.after_talk_command {
            note_talk_command();
        }
        let call = StoryCall {
            phase: item.held.phase,
            pack: 0,
            function: name.clone(),
            args: Vec::new(),
        };
        let queued = queue_call(build, &call).is_some();
        log_line(&format!("move gate: {name} queued ({queued})"));
    }
}

fn verdict_for(view: &NetShared) -> MoveVerdict {
    let partner = view.remote.as_ref().map(|remote| Companion {
        name: remote.name.clone(),
        placement: Some(Placement {
            phase: remote.state.phase,
            position: remote.state.position,
        }),
        age_ms: remote.age().as_millis() as u64,
    });
    judge_move(&MoveAsk {
        connected: view.state == NetState::Connected,
        partner_game_open: !view.partner.is_game_closed(),
        local: view.local.as_ref().map(|local| Placement {
            phase: local.phase,
            position: local.position,
        }),
        partner: partner.as_ref(),
        radius: zone_radius(),
    })
}

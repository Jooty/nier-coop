use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::stay_together::FollowRole;
use rc_game::zone_rules::{
    Companion, Placement, VetoNotice, ZoneChangeAsk, ZoneChangeSource, ZoneVerdict,
    judge_zone_change,
};

use crate::diagnostics::log_line;
use crate::net::{self, NetShared, NetState};
use crate::stay_together::note_blocked_exit;
use crate::zone_rules::settings::zone_radius;

const UNNAMED_COMPANION: &str = "the other player";

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static NOTICE: Mutex<VetoNotice> = Mutex::new(VetoNotice::new());

pub fn veto_zone_change(path: &str, source: ZoneChangeSource) -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let companions = companions_of(&view);
    let verdict = judge_zone_change(&ZoneChangeAsk {
        source,
        role: FollowRole::from_slot(view.local_slot),
        connected: view.state == NetState::Connected,
        local: view.local.as_ref().map(|local| Placement {
            phase: local.phase,
            position: local.position,
        }),
        companions: &companions,
        radius: zone_radius(),
    });
    let Some(text) = verdict.prompt_text() else {
        return false;
    };
    note_blocked_exit();
    let record = NOTICE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .record(path, &text, now_ms());
    if record.should_log {
        log_line(&format!(
            "zone rules: vetoed a zone change via {path} ({})",
            verdict_label(&verdict)
        ));
    }
    true
}

pub fn prompt_text() -> Option<String> {
    NOTICE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .visible_text(now_ms())
        .map(str::to_owned)
}

fn companions_of(view: &NetShared) -> Vec<Companion> {
    if let Some(remote) = view.remote.as_ref() {
        return vec![Companion {
            name: remote.name.clone(),
            placement: Some(Placement {
                phase: remote.state.phase,
                position: remote.state.position,
            }),
            age_ms: remote.age().as_millis() as u64,
        }];
    }
    vec![Companion {
        name: view
            .peer_name
            .clone()
            .unwrap_or_else(|| UNNAMED_COMPANION.to_owned()),
        placement: None,
        age_ms: 0,
    }]
}

fn verdict_label(verdict: &ZoneVerdict) -> &'static str {
    match verdict {
        ZoneVerdict::Allow => "allowed",
        ZoneVerdict::OnlyHostMayChange => "guest",
        ZoneVerdict::WaitingFor(_) => "host waiting for a player",
    }
}

fn now_ms() -> u64 {
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis() as u64
}

use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use rc_game::Replicant;
use rc_game::downed::{DownedRules, StepInput};
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::downed::apply::{
    OwnedBits, Pending, apply_effects, hold_endure, release_bits, resolve_pending,
};
use crate::downed::status::{DownedStatus, GO_DOWN_REQUESTED, REVIVE_REQUESTED, publish};
use crate::downed::{MAX_STEP_SECONDS, PARTNER_STATE_MAX_AGE};
use crate::game_hooks::take_lethal_hit_seen;
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared};

static RUNTIME: Mutex<Runtime> = Mutex::new(Runtime {
    rules: DownedRules::new(),
    last_tick: None,
    owned: None,
    pending: Pending {
        leave_downed: false,
        kill: false,
    },
});

struct Runtime {
    rules: DownedRules,
    last_tick: Option<Instant>,
    owned: Option<OwnedBits>,
    pending: Pending,
}

pub fn tick_downed(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let session_active = view.partner_in_play();
    let lethal_hit_seen = take_lethal_hit_seen();
    let mut runtime = RUNTIME.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    let dt_seconds = runtime.last_tick.replace(now).map_or(0.0, |last| {
        now.saturating_duration_since(last)
            .as_secs_f32()
            .min(MAX_STEP_SECONDS)
    });
    let partner_events = net.take_relayed(|message| matches!(message, Message::Combat(_)));
    let mut game = Replicant::new(InProcessMemory::new(build));
    let mut effects = Vec::new();
    if session_active {
        for message in partner_events {
            if let Message::Combat(event) = message {
                effects.extend(runtime.rules.on_partner_event(event));
            }
        }
        if GO_DOWN_REQUESTED.swap(false, Ordering::Relaxed) {
            effects.extend(runtime.rules.request_go_down());
        }
        if REVIVE_REQUESTED.swap(false, Ordering::Relaxed) {
            effects.extend(runtime.rules.request_revive_partner());
        }
    }
    if let Ok(local_health) = game.health() {
        effects.extend(runtime.rules.step(StepInput {
            dt_seconds,
            session_active,
            local_health,
            lethal_hit_seen,
            partner_distance: partner_distance(&view),
        }));
    }
    let runtime = &mut *runtime;
    apply_effects(
        net,
        &mut game,
        &mut runtime.owned,
        &mut runtime.pending,
        &effects,
    );
    resolve_pending(build, &mut game, &mut runtime.owned, &mut runtime.pending);
    maintain_endure(runtime, &mut game, session_active);
    publish(DownedStatus {
        session_active,
        local: runtime.rules.local(),
        partner: runtime.rules.partner(),
        revive_progress: runtime.rules.revive_progress(),
        bleed_out_progress: runtime.rules.bleed_out_progress(),
        game_over: runtime.rules.is_game_over(),
    });
}

fn maintain_endure(
    runtime: &mut Runtime,
    game: &mut Replicant<InProcessMemory>,
    session_active: bool,
) {
    let holding = session_active
        && !runtime.rules.is_game_over()
        && !runtime.pending.kill
        && !runtime.pending.leave_downed;
    let result = if holding {
        hold_endure(game, &mut runtime.owned)
    } else if runtime.pending.leave_downed || runtime.pending.kill {
        Ok(())
    } else {
        release_bits(game, &mut runtime.owned)
    };
    if let Err(error) = result {
        log_line(&format!("downed: life flags not writable ({error})"));
    }
}

fn partner_distance(view: &NetShared) -> Option<f32> {
    let local = view.local.as_ref()?;
    let remote = view.remote.as_ref()?;
    if remote.age() > PARTNER_STATE_MAX_AGE || remote.state.phase != local.phase {
        return None;
    }
    let squared: f32 = local
        .position
        .iter()
        .zip(remote.state.position)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    squared.is_finite().then(|| squared.sqrt())
}

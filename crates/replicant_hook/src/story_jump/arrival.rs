use std::sync::{Mutex, PoisonError};

use replicant_game::Replicant;
use replicant_game::story_jump::{
    ArrivalSpot, LEVEL_CHECK_SETTLE_MS, SET_PLAYER_POSITION_NATIVE_RVA, arrival_is_due,
    arrival_spot,
};
use replicant_proto::StoryJumpPoint;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::story_jump::natives::{NativeArg, call_native_with};

const ARRIVAL_GIVE_UP_MS: u64 = 60_000;
const READ_BACK_AFTER_MS: u64 = 1000;
const ARRIVED_WITHIN_UNITS: f32 = 1.5;

static WAITING: Mutex<Option<Waiting>> = Mutex::new(None);

struct Waiting {
    spot: ArrivalSpot,
    phase_at_apply: String,
    applied_ms: u64,
    phase_changed_ms: Option<u64>,
    field_was_left: bool,
    placed_ms: Option<u64>,
}

pub(super) fn expect_arrival(build: &VerifiedBuild, point: StoryJumpPoint, host: bool) {
    let Some(spot) = arrival_spot(point, host) else {
        return;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    *WAITING.lock().unwrap_or_else(PoisonError::into_inner) = Some(Waiting {
        spot,
        phase_at_apply: game.current_phase().unwrap_or_default(),
        applied_ms: net_clock_ms(),
        phase_changed_ms: None,
        field_was_left: false,
        placed_ms: None,
    });
}

pub(super) fn tick_arrival(build: &VerifiedBuild) {
    let mut guard = WAITING.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(waiting) = guard.as_mut() else {
        return;
    };
    let now = net_clock_ms();
    let game = Replicant::new(InProcessMemory::new(build));
    if let Some(placed_ms) = waiting.placed_ms {
        if now.saturating_sub(placed_ms) >= READ_BACK_AFTER_MS {
            let spot = waiting.spot;
            *guard = None;
            drop(guard);
            log_read_back(&game, spot);
        }
        return;
    }
    if now.saturating_sub(waiting.applied_ms) > ARRIVAL_GIVE_UP_MS {
        log_line("story jump: placing the player gave up, the map never reloaded");
        *guard = None;
        return;
    }
    let phase_now = game.current_phase().unwrap_or_default();
    let field_ready = game.map_manager_is_active().unwrap_or(false);
    if !field_ready {
        waiting.field_was_left = true;
        waiting.phase_changed_ms = None;
        return;
    }
    if phase_now == waiting.phase_at_apply && !waiting.field_was_left {
        return;
    }
    let changed_ms = *waiting.phase_changed_ms.get_or_insert(now);
    if !arrival_is_due(
        &waiting.phase_at_apply,
        &phase_now,
        waiting.field_was_left,
        field_ready,
        now.saturating_sub(changed_ms),
    ) {
        return;
    }
    waiting.placed_ms = Some(now);
    let spot = waiting.spot;
    drop(guard);
    place(build, &game, spot, &phase_now);
}

fn place(build: &VerifiedBuild, game: &Replicant<InProcessMemory>, spot: ArrivalSpot, phase: &str) {
    let heading_degrees = game.yaw_radians().map_or(0.0, f32::to_degrees);
    call_native_with(
        build,
        SET_PLAYER_POSITION_NATIVE_RVA,
        &[
            NativeArg::Real(f64::from(spot.x)),
            NativeArg::Real(f64::from(spot.y)),
            NativeArg::Real(f64::from(spot.z)),
            NativeArg::Real(f64::from(heading_degrees)),
        ],
    );
    log_line(&format!(
        "story jump: placed the player in {phase} with _SetPlayerPosition({:.1}, {:.1}, {:.1}, heading {heading_degrees:.0}) after {LEVEL_CHECK_SETTLE_MS} ms settle",
        spot.x, spot.y, spot.z
    ));
}

fn log_read_back(game: &Replicant<InProcessMemory>, spot: ArrivalSpot) {
    let Ok(here) = game.position() else {
        log_line("story jump: the player position is unreadable after placing it");
        return;
    };
    let distance = (here.x - spot.x).hypot(here.z - spot.z);
    let verdict = if distance <= ARRIVED_WITHIN_UNITS {
        "arrived"
    } else {
        "did not arrive"
    };
    log_line(&format!(
        "story jump: {verdict} (now at {:.1}, {:.1}, {:.1}, wanted {:.1}, {:.1}, {:.1})",
        here.x, here.y, here.z, spot.x, spot.y, spot.z
    ));
}

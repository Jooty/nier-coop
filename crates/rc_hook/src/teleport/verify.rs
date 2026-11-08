use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use rc_game::Replicant;
use rc_game::stay_together::{TeleportCheck, check_teleport};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::teleport::status::remember_outcome;

struct Pending {
    target: [f32; 3],
    started: Instant,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

pub(super) fn expect_arrival(target: [f32; 3]) {
    *PENDING.lock().unwrap_or_else(PoisonError::into_inner) = Some(Pending {
        target,
        started: Instant::now(),
    });
}

pub(super) fn tick(build: &VerifiedBuild) {
    let mut pending = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(waiting) = pending.as_ref() else {
        return;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    let Ok(here) = game.position() else {
        return;
    };
    let now = [here.x, here.y, here.z];
    let elapsed_ms = u64::try_from(waiting.started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let line = match check_teleport(waiting.target, now, elapsed_ms) {
        TeleportCheck::Waiting => return,
        TeleportCheck::Arrived => format!(
            "teleport: now at {:.1}, {:.1}, {:.1} after {elapsed_ms} ms",
            now[0], now[1], now[2]
        ),
        TeleportCheck::Missed => format!(
            "teleport: did not move (at {:.1}, {:.1}, {:.1}, wanted {:.1}, {:.1}, {:.1})",
            now[0], now[1], now[2], waiting.target[0], waiting.target[1], waiting.target[2]
        ),
    };
    *pending = None;
    log_line(&line);
    remember_outcome(&line);
}

use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::enemies::live_companion_texts;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

const COMPANION_LOG_INTERVAL: Duration = Duration::from_secs(2);

static LAST_LOGGED: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(Mutex::default);

pub(super) fn log_live_companions(build: &VerifiedBuild, role: &str) {
    let mut last = LAST_LOGGED.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if last.is_some_and(|at| now.saturating_duration_since(at) < COMPANION_LOG_INTERVAL) {
        return;
    }
    *last = Some(now);
    for text in live_companion_texts(&InProcessMemory::new(build)) {
        log_line(&format!("companion ({role}): {text}"));
    }
}

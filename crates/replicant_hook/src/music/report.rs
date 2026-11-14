use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};

use nc_hook::line_budget::LineBudget;

use crate::diagnostics::log_line;
use crate::music::{LOG_LINES_PER_WINDOW, LOG_WINDOW_MS};
use crate::net::net_clock_ms;

static IGNORED_CALLS: AtomicU32 = AtomicU32::new(0);
static LOG_WINDOW: Mutex<LineBudget> = Mutex::new(LineBudget::counting_from_zero());

pub(super) fn count_ignored_call() {
    IGNORED_CALLS.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn take_ignored_calls() -> u32 {
    IGNORED_CALLS.swap(0, Ordering::Relaxed)
}

pub(super) fn log_limited(line: &str) {
    let admitted = LOG_WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .admit(net_clock_ms(), LOG_WINDOW_MS, LOG_LINES_PER_WINDOW);
    if admitted {
        log_line(line);
    }
}

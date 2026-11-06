use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::event_log::{EventLog, VisibleEvent};

use crate::net::net_clock_ms;

static LOG: Mutex<EventLog> = Mutex::new(EventLog::new());
static ENABLED: AtomicBool = AtomicBool::new(true);

pub fn post_event(text: &str) {
    let posted = LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .post(text, net_clock_ms());
    if posted {
        log_posted(text);
    }
}

pub fn post_event_unlogged(text: &str) {
    LOG.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .post(text, net_clock_ms());
}

pub fn take_visible_events() -> Vec<VisibleEvent> {
    LOG.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .visible(net_clock_ms())
}

pub fn event_log_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_event_log_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

#[cfg(windows)]
fn log_posted(text: &str) {
    crate::diagnostics::log_line(&format!("event log: {text}"));
}

#[cfg(not(windows))]
fn log_posted(_text: &str) {}

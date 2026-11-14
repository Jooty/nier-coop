use std::sync::Mutex;

use automata_game::minute_line_budget::{LineAdmission, MinuteLineBudget};
use nc_hook::diagnostics::log_line;
use nc_hook::log_stamp::unix_millis_now;

pub fn log_owned_within(budget: &mut MinuteLineBudget, topic: &str, line: String) {
    let admission = budget.admit(unix_millis_now(), topic);
    log_held_back_note(&admission);
    if admission.allowed {
        log_line(&line);
    }
}

pub fn log_within(budget: &Mutex<MinuteLineBudget>, topic: &str, line: impl FnOnce() -> String) {
    let admission = match budget.lock() {
        Ok(mut budget) => budget.admit(unix_millis_now(), topic),
        Err(_) => return,
    };
    if let Some(note) = admission.held_back_note {
        log_line(&note);
    }
    if admission.allowed {
        log_line(&line());
    }
}

pub fn log_held_back_note(admission: &LineAdmission) {
    if let Some(note) = &admission.held_back_note {
        log_line(note);
    }
}

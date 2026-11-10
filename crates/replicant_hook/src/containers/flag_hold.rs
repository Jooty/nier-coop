use std::sync::PoisonError;

use replicant_game::story_progress::StoryArrays;
use replicant_proto::StoryChange;

use crate::containers::state::{HELD, now_ms};
use crate::diagnostics::log_line;

pub fn keep_held_flags(fixes: Vec<StoryChange>, host: Option<&StoryArrays>) -> Vec<StoryChange> {
    let mut hold = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    if hold.is_empty() {
        return fixes;
    }
    let outcome = hold.filter(
        fixes,
        |array, id| host.is_some_and(|host| host.flag(array, id)),
        now_ms(),
    );
    for flag in &outcome.released {
        log_line(&format!(
            "containers: host state has {:?} flag {}, hold released",
            flag.array, flag.id
        ));
    }
    for flag in &outcome.expired {
        log_line(&format!(
            "containers: host state never got {:?} flag {} in time, hold given up",
            flag.array, flag.id
        ));
    }
    outcome.fixes
}

pub fn expire_held_flags(host: Option<&StoryArrays>) {
    keep_held_flags(Vec::new(), host);
}

pub fn clear_held_flags() {
    let mut hold = HELD.lock().unwrap_or_else(PoisonError::into_inner);
    if !hold.is_empty() {
        hold.clear();
        log_line("containers: partner left, container flag holds cleared");
    }
}

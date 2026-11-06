use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

const REMEMBERED_LINE_LIMIT: usize = 500;

static REMEMBERED_LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn recent_lines(count: usize) -> Vec<String> {
    let lines = REMEMBERED_LINES
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let skipped = lines.len().saturating_sub(count);
    lines.iter().skip(skipped).cloned().collect()
}

pub(super) fn remember_line(message: &str) {
    let mut lines = REMEMBERED_LINES
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if lines.len() == REMEMBERED_LINE_LIMIT {
        lines.pop_front();
    }
    lines.push_back(message.to_owned());
}

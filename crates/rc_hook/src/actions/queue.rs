use std::collections::VecDeque;
use std::fmt;
use std::sync::{Mutex, PoisonError};

use crate::actions::MAX_QUEUED_ACTIONS;
use crate::actions::action::Action;

static QUEUED_ACTIONS: Mutex<VecDeque<Action>> = Mutex::new(VecDeque::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull;

impl fmt::Display for QueueFull {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "action queue holds {MAX_QUEUED_ACTIONS} already")
    }
}

impl std::error::Error for QueueFull {}

pub fn enqueue(action: Action) -> Result<(), QueueFull> {
    let mut queue = QUEUED_ACTIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if queue.len() >= MAX_QUEUED_ACTIONS {
        return Err(QueueFull);
    }
    queue.push_back(action);
    Ok(())
}

pub(super) fn take_all() -> Vec<Action> {
    let mut queue = QUEUED_ACTIONS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    queue.drain(..).collect()
}

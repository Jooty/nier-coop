use nt_hook::actions::{ActionQueue, QueueFull};

use crate::actions::{Action, MAX_QUEUED_ACTIONS};

static QUEUED_ACTIONS: ActionQueue<Action> = ActionQueue::new(MAX_QUEUED_ACTIONS);

pub fn enqueue(action: Action) -> Result<(), QueueFull> {
    QUEUED_ACTIONS.enqueue(action)
}

pub(super) fn take_all() -> Vec<Action> {
    QUEUED_ACTIONS.take_all()
}

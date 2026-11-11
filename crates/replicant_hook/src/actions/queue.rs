use nc_hook::actions::ActionQueue;
pub use nc_hook::actions::QueueFull;

use crate::actions::MAX_QUEUED_ACTIONS;
use crate::actions::action::Action;

static QUEUED_ACTIONS: ActionQueue<Action> = ActionQueue::new(MAX_QUEUED_ACTIONS);

pub fn enqueue(action: Action) -> Result<(), QueueFull> {
    QUEUED_ACTIONS.enqueue(action)
}

pub(super) fn take_all() -> Vec<Action> {
    QUEUED_ACTIONS.take_all()
}

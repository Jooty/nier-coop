use crate::fight_sync::{
    CALLBACK_REGISTRY_ARGUMENTS_FIRST_OFFSET, CALLBACK_REGISTRY_ENTRY_COUNT,
    CALLBACK_REGISTRY_ENTRY_STRIDE, CALLBACK_REGISTRY_KIND_FIRST_OFFSET, PopSource,
    SCRIPT_SLOT_STEPPING_HELD_STATE, SCRIPT_SLOT_WAITING_TO_POP_STATES, SlotState,
};
use crate::script_runner::{QUEUE_FIRST_SLOT_OFFSET, QUEUE_SLOT_COUNT, QUEUE_SLOT_STRIDE};
use crate::story_relay::QUEUED_CALL_ARGUMENTS_OFFSET;

/// Which list the pop function (`0x413260`) took a call from, told by the argument pointer it
/// stored: a fired registry entry's block or a queue slot's block.
pub fn pop_source(manager: u64, arguments: u64) -> Option<PopSource> {
    if let Some(offset) = arguments.checked_sub(manager + CALLBACK_REGISTRY_ARGUMENTS_FIRST_OFFSET)
        && offset < CALLBACK_REGISTRY_ENTRY_COUNT * CALLBACK_REGISTRY_ENTRY_STRIDE
        && offset % CALLBACK_REGISTRY_ENTRY_STRIDE == 0
    {
        return Some(PopSource::Fired);
    }
    let queue_arguments = manager + QUEUE_FIRST_SLOT_OFFSET + QUEUED_CALL_ARGUMENTS_OFFSET;
    if let Some(offset) = arguments.checked_sub(queue_arguments)
        && offset < QUEUE_SLOT_COUNT * QUEUE_SLOT_STRIDE
        && offset % QUEUE_SLOT_STRIDE == 0
    {
        return Some(PopSource::Queued);
    }
    None
}

pub fn callback_registry_kind_address(manager: u64, index: u64) -> u64 {
    manager + CALLBACK_REGISTRY_KIND_FIRST_OFFSET + index * CALLBACK_REGISTRY_ENTRY_STRIDE
}

pub const fn is_registered_callback_kind(kind: i32) -> bool {
    kind >= 0
}

/// The script slot as the phase script manager's state shows it; an unreadable state counts as
/// busy, so nothing is queued while the manager is missing.
pub fn script_slot_state(manager_state: Option<i32>) -> SlotState {
    match manager_state {
        Some(state) if SCRIPT_SLOT_WAITING_TO_POP_STATES.contains(&state) => SlotState::Idle,
        Some(SCRIPT_SLOT_STEPPING_HELD_STATE) => SlotState::SteppingHeld,
        Some(_) | None => SlotState::Busy,
    }
}

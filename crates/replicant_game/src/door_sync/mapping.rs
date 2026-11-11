use replicant_proto::DoorAction;

use crate::door_sync::{
    DOOR_ACTIVE_MESSAGE_ID, DOOR_CLOSE_MESSAGE_ID, DOOR_LOCK_MESSAGE_ID, DOOR_OPEN_MESSAGE_ID,
};

pub fn action_of_message(message: u32) -> Option<DoorAction> {
    match message {
        DOOR_LOCK_MESSAGE_ID => Some(DoorAction::Lock),
        DOOR_OPEN_MESSAGE_ID => Some(DoorAction::Open),
        DOOR_CLOSE_MESSAGE_ID => Some(DoorAction::Close),
        DOOR_ACTIVE_MESSAGE_ID => Some(DoorAction::Active),
        _ => None,
    }
}

pub fn message_of_action(action: DoorAction) -> u32 {
    match action {
        DoorAction::Lock => DOOR_LOCK_MESSAGE_ID,
        DoorAction::Open => DOOR_OPEN_MESSAGE_ID,
        DoorAction::Close => DOOR_CLOSE_MESSAGE_ID,
        DoorAction::Active => DOOR_ACTIVE_MESSAGE_ID,
    }
}

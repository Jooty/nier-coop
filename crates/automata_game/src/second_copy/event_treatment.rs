use crate::second_copy::{ERROR_ALREADY_EXISTS, GAME_EVENT_NAME};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTreatment {
    PassThrough,
    RenameWhenAlreadyExists,
}

pub fn event_treatment(name: Option<&[u8]>) -> EventTreatment {
    match name {
        Some(bytes) if bytes == GAME_EVENT_NAME.as_bytes() => {
            EventTreatment::RenameWhenAlreadyExists
        }
        _ => EventTreatment::PassThrough,
    }
}

pub fn must_rename(treatment: EventTreatment, last_error: u32) -> bool {
    treatment == EventTreatment::RenameWhenAlreadyExists && last_error == ERROR_ALREADY_EXISTS
}

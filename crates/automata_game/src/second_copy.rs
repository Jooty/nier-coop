mod event_treatment;
mod unique_event_name;

pub use event_treatment::{EventTreatment, event_treatment, must_rename};
pub use unique_event_name::unique_event_name;

pub const GAME_EVENT_NAME: &str = "NieR:AutomataEvent";
pub const ERROR_ALREADY_EXISTS: u32 = 0xB7;

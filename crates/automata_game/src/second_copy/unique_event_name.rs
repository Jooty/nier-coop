use crate::second_copy::GAME_EVENT_NAME;

pub fn unique_event_name(process_id: u32) -> String {
    format!("{GAME_EVENT_NAME}.{process_id}")
}

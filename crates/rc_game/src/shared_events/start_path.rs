use crate::callback_gate::{event_base, is_gated_callback, is_scene_talk};
use crate::phases::PHASE_TABLE;
use crate::shared_events::SHARED_EVENT_BASES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPath {
    YourTalk,
    SharedEvent,
    SceneTalk,
    Cutscene,
}

pub fn phase_name(phase: u16) -> Option<&'static str> {
    PHASE_TABLE.get(usize::from(phase)).map(|entry| entry.name)
}

pub fn is_shared_event(phase: &str, name: &str) -> bool {
    is_gated_callback(name)
        && SHARED_EVENT_BASES
            .binary_search_by(|(table_phase, table_base)| {
                (*table_phase, *table_base).cmp(&(phase, event_base(name)))
            })
            .is_ok()
}

pub fn start_path(phase: Option<&str>, name: &str, after_talk_command: bool) -> StartPath {
    if after_talk_command {
        StartPath::YourTalk
    } else if phase.is_some_and(|phase| is_shared_event(phase, name)) {
        StartPath::SharedEvent
    } else if is_scene_talk(name) {
        StartPath::SceneTalk
    } else {
        StartPath::Cutscene
    }
}

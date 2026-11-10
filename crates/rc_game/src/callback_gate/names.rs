use crate::callback_gate::{
    GATED_CALLBACK_NAMES, GATED_EVENT_SUFFIX, SCENE_TALK_PREFIX, TALK_EVENT_SUFFIX,
};
use crate::story_calls::is_event_callback_name;

pub fn is_gated_callback(name: &str) -> bool {
    if GATED_CALLBACK_NAMES.contains(&name) {
        return true;
    }
    is_event_callback_name(name)
        && name
            .trim_end_matches(|character: char| character.is_ascii_digit())
            .ends_with(GATED_EVENT_SUFFIX)
}

pub fn is_scene_talk(name: &str) -> bool {
    name.starts_with(SCENE_TALK_PREFIX) && is_gated_callback(name)
}

pub fn is_event_talk(name: &str) -> bool {
    (name.starts_with("EID_") || name.starts_with("SEID_")) && name.ends_with(TALK_EVENT_SUFFIX)
}

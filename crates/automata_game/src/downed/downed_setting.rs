use crate::downed::{DOWNED_REVIVE_SETTING_OFF_TEXT, DOWNED_REVIVE_SETTING_ON_TEXT};

/// None for a text that is neither "on" nor "off".
pub fn downed_revive_from_text(text: &str) -> Option<bool> {
    match text {
        DOWNED_REVIVE_SETTING_ON_TEXT => Some(true),
        DOWNED_REVIVE_SETTING_OFF_TEXT => Some(false),
        _ => None,
    }
}

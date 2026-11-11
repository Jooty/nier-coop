use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerSettings {
    pub keep_running: bool,
    pub event_log: bool,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            keep_running: true,
            event_log: true,
        }
    }
}

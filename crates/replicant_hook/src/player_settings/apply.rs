use replicant_proto::PlayerSettings;

use crate::event_log::{event_log_enabled, set_event_log_enabled};
use crate::game_hooks::{keep_running_enabled, set_keep_running};

pub fn current() -> PlayerSettings {
    PlayerSettings {
        keep_running: keep_running_enabled(),
        event_log: event_log_enabled(),
    }
}

pub fn apply(settings: PlayerSettings) {
    set_keep_running(settings.keep_running);
    set_event_log_enabled(settings.event_log);
}

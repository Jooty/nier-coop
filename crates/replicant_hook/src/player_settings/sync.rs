use replicant_proto::SessionOptions;

use crate::diagnostics::log_line;
use crate::net::handle;
use crate::player_settings::apply::{apply, current};

pub fn take_from_launcher() {
    let Some(settings) = handle().and_then(|net| net.take_launcher_settings()) else {
        return;
    };
    apply(settings);
    log_line(&format!(
        "settings: the launcher changed them to {settings:?}"
    ));
}

pub fn publish_to_launcher() {
    let settings = current();
    log_line(&format!(
        "settings: changed in game, telling the launcher {settings:?}"
    ));
    if let Some(net) = handle() {
        net.tell_launcher_settings(settings);
    }
}

pub fn publish_host_options(options: SessionOptions) {
    log_line(&format!(
        "settings: host options changed in game {options:?}"
    ));
    if let Some(net) = handle() {
        net.tell_launcher_host_options(options);
    }
}

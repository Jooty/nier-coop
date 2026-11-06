use hudhook::imgui::Ui;
use rc_proto::SessionOptions;

use crate::net::{self, NetState};
use crate::player_settings::{apply, current, publish_host_options, publish_to_launcher};

pub(super) fn draw_settings_section(ui: &Ui) {
    let before = current();
    let mut settings = before;
    ui.checkbox("Keep running in the background", &mut settings.keep_running);
    ui.checkbox("Show the event log", &mut settings.event_log);
    if settings != before {
        apply(settings);
        publish_to_launcher();
    }
    ui.separator();
    let view = net::handle().map(|net| net.view());
    draw_host_only_options(
        ui,
        view.as_ref().and_then(|view| view.local_slot),
        view.as_ref().map(|view| view.options),
    );
    let linked = view
        .as_ref()
        .is_some_and(|view| view.state != NetState::WaitingForLauncher);
    ui.text_disabled(if linked {
        "Shared with the launcher: change them here or there."
    } else {
        "The launcher is not linked: changes last until the game closes."
    });
}

fn draw_host_only_options(ui: &Ui, slot: Option<u8>, options: Option<SessionOptions>) {
    let options = options.unwrap_or_default();
    if slot == Some(0) {
        let mut edited = options;
        ui.checkbox("Show names over players", &mut edited.show_names);
        ui.checkbox("Show party frame", &mut edited.show_party_frame);
        ui.checkbox("Show distance", &mut edited.show_distance);
        ui.checkbox("Show ping", &mut edited.show_ping);
        if edited != options {
            publish_host_options(edited);
        }
        return;
    }
    ui.text_disabled(format!(
        "Show names over players: {}",
        on_or_off(options.show_names)
    ));
    ui.text_disabled(format!(
        "Show party frame: {}",
        on_or_off(options.show_party_frame)
    ));
    ui.text_disabled(format!(
        "Show distance: {}",
        on_or_off(options.show_distance)
    ));
    ui.text_disabled(format!("Show ping: {}", on_or_off(options.show_ping)));
    ui.text_disabled(if slot.is_some() {
        "These are the host's choice."
    } else {
        "The host chooses these once someone joins."
    });
}

fn on_or_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

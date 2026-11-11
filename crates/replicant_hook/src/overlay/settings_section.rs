use hudhook::imgui::Ui;
use replicant_proto::{MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT, SessionOptions};

use crate::net::{self, NetState};
use crate::paused_hits::{paused_players_can_be_hit, set_paused_players_can_be_hit};
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
        edited.enemy_health_percent = percent_slider(
            ui,
            "Enemy health per extra player",
            edited.enemy_health_percent,
            MAX_ENEMY_HEALTH_PERCENT,
        );
        edited.enemy_damage_percent = percent_slider(
            ui,
            "Enemy damage per extra player",
            edited.enemy_damage_percent,
            MAX_ENEMY_DAMAGE_PERCENT,
        );
        if edited != options {
            publish_host_options(edited);
        }
        let mut hittable = paused_players_can_be_hit();
        if ui.checkbox("Paused players can be hit", &mut hittable) {
            set_paused_players_can_be_hit(hittable);
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
    ui.text_disabled(format!(
        "Enemy health per extra player: +{}%",
        options.enemy_health_percent
    ));
    ui.text_disabled(format!(
        "Enemy damage per extra player: +{}%",
        options.enemy_damage_percent
    ));
    ui.text_disabled(format!(
        "Paused players can be hit: {}",
        on_or_off(paused_players_can_be_hit())
    ));
    ui.text_disabled(if slot.is_some() {
        "These are the host's choice."
    } else {
        "The host chooses these once someone joins."
    });
}

fn on_or_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

fn percent_slider(ui: &Ui, label: &str, value: u16, max: u16) -> u16 {
    let mut shown = i32::from(value);
    ui.slider(label, 0, i32::from(max), &mut shown);
    u16::try_from(shown).unwrap_or(value).min(max)
}

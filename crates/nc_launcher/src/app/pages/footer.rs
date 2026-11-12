use eframe::egui::Ui;

use crate::app::style::{BAD, GOOD, WARN};
use crate::app::{Launcher, NetForms, UpdateState};
use crate::net::NetState;

pub(super) fn draw_footer(ui: &mut Ui, launcher: &mut Launcher, forms: &NetForms) {
    draw_status_line(ui, launcher, forms);
}

fn draw_status_line(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    ui.horizontal_wrapped(|ui| {
        status_item(
            ui,
            "game",
            launcher.own_game_running,
            "running",
            "not running",
        );
        status_item(ui, "link", launcher.link.game_linked, "up", "down");
        match &state {
            NetState::Connected { peer_name } => {
                ui.colored_label(GOOD, format!("peer: {peer_name}"));
                if launcher.lobby_started && !launcher.link.partner_readiness().is_in_play() {
                    ui.colored_label(WARN, waiting_for_game_text(launcher, peer_name));
                }
            }
            NetState::Hosting { .. } => {
                ui.colored_label(WARN, "peer: waiting");
            }
            NetState::Joining => {
                ui.colored_label(WARN, "peer: joining");
            }
            NetState::Failed(reason) => {
                ui.colored_label(BAD, format!("failed: {reason}"));
            }
            NetState::Idle => {
                ui.label("peer: none");
            }
        }
        if let UpdateState::Available(update) = &launcher.updater.state {
            ui.colored_label(
                WARN,
                format!(
                    "update available: {} (Settings)",
                    crate::update::short_commit(&update.manifest.commit)
                ),
            );
        }
        ui.label(crate::update::own_channel_label());
        match launcher.link.rtt {
            Some(rtt) => ui.label(format!("ping: {} ms", rtt.as_millis())),
            None => ui.label("ping: -"),
        };
    });
}

fn waiting_for_game_text(launcher: &Launcher, peer_name: &str) -> String {
    if launcher.link.ticket.is_some() {
        format!("Waiting for {peer_name}'s game...")
    } else {
        "Waiting for the host's game...".to_owned()
    }
}

fn status_item(ui: &mut Ui, label: &str, up: bool, up_text: &str, down_text: &str) {
    if up {
        ui.colored_label(GOOD, format!("{label}: {up_text}"));
    } else {
        ui.label(format!("{label}: {down_text}"));
    }
}

use eframe::egui::{self, RichText, Ui};

use crate::app::style::{BAD, GOOD, WARN};
use crate::app::{Launcher, NetForms, UpdateState};
use crate::net::NetState;

const FOOTER_LOG_HEIGHT: f32 = 80.0;

pub(super) fn draw_footer(ui: &mut Ui, launcher: &mut Launcher, forms: &NetForms) {
    draw_status_line(ui, launcher, forms);
    ui.horizontal(|ui| {
        if ui.button("Copy log").clicked() {
            ui.ctx().copy_text(launcher.log.join("\n"));
        }
        if ui.button("Export logs").clicked() {
            launcher.export_logs();
        }
        if launcher.log_upload_available()
            && ui
                .add_enabled(
                    !launcher.log_upload_busy(),
                    egui::Button::new("Send logs now"),
                )
                .clicked()
        {
            launcher.send_logs_now(ui.ctx());
        }
        if ui.button("Open logs folder").clicked() {
            launcher.open_logs_folder();
        }
        if let Some(note) = &launcher.export_note {
            ui.label(RichText::new(note).weak());
        }
    });
    egui::ScrollArea::vertical()
        .max_height(FOOTER_LOG_HEIGHT)
        .auto_shrink([false, true])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.set_max_width(ui.available_width());
            if launcher.log.is_empty() {
                ui.label(RichText::new("nothing logged yet").weak());
            }
            for line in &launcher.log {
                ui.add(egui::Label::new(RichText::new(line).monospace()).wrap());
            }
        });
}

fn draw_status_line(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    ui.horizontal_wrapped(|ui| {
        status_item(ui, "game", launcher.game_running, "running", "not running");
        status_item(ui, "link", launcher.link.game_linked, "up", "down");
        match &state {
            NetState::Connected { peer_name } => {
                ui.colored_label(GOOD, format!("peer: {peer_name}"));
                if launcher.lobby_started && !launcher.link.peer_readiness.is_in_play() {
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

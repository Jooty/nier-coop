use eframe::egui::{Align, Color32, Label, Layout, RichText, Ui};

use crate::app::pages::marks::status_dot;
use crate::app::style::{BAD, GOOD, MUTED, WARN, small_font};
use crate::app::{Launcher, NetForms, UpdateState};
use crate::net::NetState;

pub(super) const STATUS_LINE_HEIGHT_PX: f32 = 28.0;
const STATUS_ITEM_GAP_PX: f32 = 22.0;
const STATUS_DOT_TO_TEXT_GAP_PX: f32 = 7.0;

pub(super) fn draw_footer(ui: &mut Ui, launcher: &mut Launcher, forms: &NetForms) {
    draw_status_line(ui, launcher, forms);
}

fn draw_status_line(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        on_off_item(
            ui,
            "game",
            launcher.own_game_running,
            "running",
            "not running",
        );
        on_off_item(ui, "link", launcher.link.game_linked, "up", "down");
        match &state {
            NetState::Connected { peer_name } => {
                dotted_item(ui, GOOD, &format!("peer: {peer_name}"));
                if launcher.lobby_started && !launcher.link.partner_readiness().is_in_play() {
                    dotted_item(ui, WARN, &waiting_for_game_text(launcher, peer_name));
                }
            }
            NetState::Hosting { .. } => dotted_item(ui, WARN, "peer: waiting"),
            NetState::Joining => dotted_item(ui, WARN, "peer: joining"),
            NetState::Failed(reason) => dotted_item(ui, BAD, &format!("failed: {reason}")),
            NetState::Idle => dotted_item(ui, MUTED, "peer: none"),
        }
        if let UpdateState::Available(update) = &launcher.updater.state {
            dotted_item(
                ui,
                WARN,
                &format!(
                    "update available: {} (Settings)",
                    crate::update::short_commit(&update.manifest.commit)
                ),
            );
        }
        plain_item(ui, &crate::update::own_channel_label());
        let ping = match launcher.link.rtt {
            Some(rtt) => format!("ping: {} ms", rtt.as_millis()),
            None => "ping: -".to_owned(),
        };
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            status_text(ui, MUTED, &ping);
        });
    });
}

fn waiting_for_game_text(launcher: &Launcher, peer_name: &str) -> String {
    if launcher.link.ticket.is_some() {
        format!("Waiting for {peer_name}'s game...")
    } else {
        "Waiting for the host's game...".to_owned()
    }
}

fn on_off_item(ui: &mut Ui, label: &str, up: bool, up_text: &str, down_text: &str) {
    if up {
        dotted_item(ui, GOOD, &format!("{label}: {up_text}"));
    } else {
        dotted_item(ui, MUTED, &format!("{label}: {down_text}"));
    }
}

fn dotted_item(ui: &mut Ui, color: Color32, text: &str) {
    status_dot(ui, color);
    ui.add_space(STATUS_DOT_TO_TEXT_GAP_PX);
    status_text(ui, color, text);
    ui.add_space(STATUS_ITEM_GAP_PX);
}

fn plain_item(ui: &mut Ui, text: &str) {
    status_text(ui, MUTED, text);
    ui.add_space(STATUS_ITEM_GAP_PX);
}

fn status_text(ui: &mut Ui, color: Color32, text: &str) {
    ui.add(
        Label::new(RichText::new(text).font(small_font()).color(color))
            .selectable(false)
            .extend(),
    );
}

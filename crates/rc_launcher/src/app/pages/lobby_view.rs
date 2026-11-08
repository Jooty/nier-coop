use eframe::egui::{self, Ui};
use rc_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::pages::lobby::{draw_game_state, draw_ready};
use crate::lobby::{
    HOST_CHARACTER_TOOLTIP, PlayerRole, PlayerRow, RowInputs, body_label, picker_choices,
    player_rows,
};

pub(super) fn draw_player_list(ui: &mut Ui, launcher: &mut Launcher, peer_name: Option<&str>) {
    let viewer_is_guest = launcher.link.guest;
    let own_name = launcher.effective_player_name();
    let guest_body = guest_body_shown(launcher, viewer_is_guest);
    let rows = player_rows(RowInputs {
        viewer_is_guest,
        own_name: &own_name,
        peer_name,
        guest_body,
    });
    let own_ready = launcher.own_lobby_ready();
    let peer_ready = launcher.link.peer_lobby;
    let connected = peer_name.is_some();
    let mut picked = None;
    egui::Grid::new("lobby-players")
        .num_columns(5)
        .spacing([16.0, 6.0])
        .show(ui, |ui| {
            for row in &rows {
                draw_name(ui, row);
                ui.label(row.role.text());
                picked = picked.or(draw_character(ui, row));
                if connected {
                    draw_ready(ui, if row.you { Some(own_ready) } else { peer_ready });
                    let state = if row.you {
                        launcher.your_game_state()
                    } else {
                        launcher.their_game_state()
                    };
                    draw_game_state(ui, state);
                }
                ui.end_row();
            }
        });
    if let Some(body) = picked {
        launcher.set_body(body);
    }
}

fn guest_body_shown(launcher: &mut Launcher, viewer_is_guest: bool) -> PartnerBody {
    if !viewer_is_guest {
        return launcher.link.peer_body.unwrap_or_default();
    }
    let own = launcher.own_lobby_body();
    if !picker_choices().contains(&own) {
        launcher.set_body(PartnerBody::Nier);
        return PartnerBody::Nier;
    }
    own
}

fn draw_name(ui: &mut Ui, row: &PlayerRow) {
    if row.you {
        ui.label(format!("{} (you)", row.name));
    } else {
        ui.label(&row.name);
    }
}

fn draw_character(ui: &mut Ui, row: &PlayerRow) -> Option<PartnerBody> {
    if row.picker {
        let mut body = row.body;
        egui::ComboBox::from_id_salt("guest-character")
            .selected_text(body_label(body))
            .show_ui(ui, |ui| {
                for choice in picker_choices() {
                    ui.selectable_value(&mut body, *choice, body_label(*choice));
                }
            });
        return (body != row.body).then_some(body);
    }
    let label = ui.label(body_label(row.body));
    if row.role == PlayerRole::Host {
        label.on_hover_text(HOST_CHARACTER_TOOLTIP);
    }
    None
}

use eframe::egui::{self, Ui};
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::style::{ACCENT, BAD, GOOD, INFO, LINE, MUTED, WARN, heading_text};
use crate::lobby::{
    HOST_CHARACTER_TOOLTIP, PLAYER_TABLE_HEADERS, PlayerRole, PlayerRow, PlayerStatus, RowInputs,
    body_label, picker_choices, player_rows,
};

const NAME_COLUMN_WIDTH: f32 = 150.0;
const PLAYER_TABLE_COLUMN_SPACING: f32 = 16.0;

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
    let choices = picker_choices(launcher.profile, &[PartnerBody::Nier]);
    let mut picked = None;
    let grid = egui::Grid::new("lobby-players")
        .num_columns(4)
        .spacing([PLAYER_TABLE_COLUMN_SPACING, 6.0])
        .show(ui, |ui| {
            let mut header_bottom = 0.0;
            for header in PLAYER_TABLE_HEADERS {
                let label = ui.label(heading_text(header).color(MUTED));
                header_bottom = label.rect.bottom();
            }
            ui.end_row();
            for row in &rows {
                draw_name(ui, row);
                ui.label(row.role.text());
                picked = picked.or(draw_character(ui, row, &choices));
                let status = if row.you {
                    launcher.your_player_status()
                } else {
                    launcher.their_player_status()
                };
                draw_status(ui, status);
                ui.end_row();
            }
            header_bottom
        });
    let rule_y = grid.inner + 3.0;
    let rect = grid.response.rect;
    ui.painter()
        .hline(rect.x_range(), rule_y, egui::Stroke::new(1.0, LINE));
    if let Some(body) = picked {
        launcher.set_body(body);
    }
    draw_follow_notice(ui, launcher);
}

fn draw_follow_notice(ui: &mut Ui, launcher: &mut Launcher) {
    let Some(notice) = launcher.follow_notice else {
        return;
    };
    ui.horizontal(|ui| {
        ui.colored_label(WARN, notice.text());
        if launcher.can_relaunch_and_rejoin() && ui.button("Relaunch and rejoin").clicked() {
            launcher.relaunch_and_rejoin();
        }
    });
}

fn guest_body_shown(launcher: &mut Launcher, viewer_is_guest: bool) -> PartnerBody {
    if !viewer_is_guest {
        return launcher.link.peer_body.unwrap_or_default();
    }
    let own = launcher.own_lobby_body();
    if !picker_choices(launcher.profile, &[PartnerBody::Nier]).contains(&own) {
        launcher.set_body(PartnerBody::Nier);
        return PartnerBody::Nier;
    }
    own
}

fn draw_name(ui: &mut Ui, row: &PlayerRow) {
    let mut text = egui::RichText::new(&row.name);
    if row.you {
        text = text.color(ACCENT);
    }
    ui.add_sized(
        [NAME_COLUMN_WIDTH, ui.spacing().interact_size.y],
        egui::Label::new(text).truncate(),
    )
    .on_hover_text(&row.name);
}

fn draw_status(ui: &mut Ui, status: PlayerStatus) {
    let color = match status {
        PlayerStatus::InGame | PlayerStatus::Ready => GOOD,
        PlayerStatus::StartingGame => INFO,
        PlayerStatus::GameClosed => BAD,
        PlayerStatus::NotReady => MUTED,
    };
    ui.add(egui::Label::new(egui::RichText::new(status.text()).color(color)).extend());
}

fn draw_character(ui: &mut Ui, row: &PlayerRow, choices: &[PartnerBody]) -> Option<PartnerBody> {
    if row.picker {
        let mut body = row.body;
        egui::ComboBox::from_id_salt("guest-character")
            .selected_text(body_label(body))
            .show_ui(ui, |ui| {
                for choice in choices {
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

use eframe::egui::{self, Ui};
use nc_launcher_core::{Character, GameProfile};
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::style::{ACCENT, BAD, GOOD, INFO, LINE, MUTED, WARN, heading_text};
use crate::lobby::{
    HOST_CHARACTER_TOOLTIP, PLAYER_TABLE_HEADERS, PlayerRole, PlayerRow, PlayerStatus, RowInputs,
    body_label, character_label_of, lobby_picks_character_id, picker_choices, player_rows,
};

const PARTNER_CHARACTER_UNSEEN_TEXT: &str = "chosen on their side";
const NAME_COLUMN_WIDTH: f32 = 150.0;
const PLAYER_TABLE_COLUMN_SPACING: f32 = 16.0;

pub(super) fn draw_player_list(ui: &mut Ui, launcher: &mut Launcher, peer_name: Option<&str>) {
    let viewer_is_guest = launcher.link.guest;
    let own_name = launcher.effective_player_name();
    let picks_by_id = lobby_picks_character_id(launcher.profile);
    let guest_body = if picks_by_id {
        PartnerBody::Nier
    } else {
        guest_body_shown(launcher, viewer_is_guest)
    };
    let rows = player_rows(RowInputs {
        viewer_is_guest,
        own_name: &own_name,
        peer_name,
        guest_body,
    });
    let choices = picker_choices(launcher.profile, &[PartnerBody::Nier]);
    let mut picked = None;
    let mut picked_id = None;
    let own_character_id = launcher.own_character_id(viewer_is_guest);
    let partner_character_id = launcher.partner_character_id(viewer_is_guest);
    let own_choices = launcher.own_character_choices(viewer_is_guest);
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
                if picks_by_id {
                    picked_id = picked_id.or(draw_character_by_id(
                        ui,
                        launcher.profile,
                        row,
                        CharacterColumn {
                            own_id: own_character_id,
                            partner_id: partner_character_id,
                            own_choices: &own_choices,
                        },
                    ));
                } else {
                    picked = picked.or(draw_character(ui, row, &choices));
                }
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
    if let Some(id) = picked_id {
        launcher.set_character_id(id);
    }
    if picks_by_id && let Some(notice) = launcher.character_bump_notice() {
        ui.colored_label(WARN, notice);
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
        return launcher
            .link
            .partner()
            .and_then(|partner| partner.body)
            .unwrap_or_default();
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

struct CharacterColumn<'a> {
    own_id: &'static str,
    partner_id: Option<&'static str>,
    own_choices: &'a [&'static Character],
}

fn draw_character_by_id(
    ui: &mut Ui,
    profile: &'static GameProfile,
    row: &PlayerRow,
    column: CharacterColumn,
) -> Option<&'static str> {
    if row.you {
        let mut id = column.own_id;
        egui::ComboBox::from_id_salt("own-character")
            .selected_text(character_label_of(profile, id))
            .show_ui(ui, |ui| {
                for choice in column.own_choices {
                    ui.selectable_value(&mut id, choice.id, choice.label);
                }
            });
        return (id != column.own_id).then_some(id);
    }
    let text = column
        .partner_id
        .map_or(PARTNER_CHARACTER_UNSEEN_TEXT, |id| {
            character_label_of(profile, id)
        });
    ui.label(text);
    None
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

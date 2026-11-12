use eframe::egui::Ui;

use crate::app::Launcher;
use crate::app::parts::{MenuValue, menu_row, section_heading};

const PLAYERS_HEADING: &str = "Players";
const HEADING_TO_LIST_GAP_PX: f32 = 10.0;
const READY_TEXT: &str = "Ready";
const NOT_READY_TEXT: &str = "Not ready";

pub(super) fn draw_member_list(ui: &mut Ui, launcher: &Launcher) {
    let party = 1 + launcher.link.members.len();
    let note = format!("{party} / {}", launcher.link.rules.max_players);
    section_heading(ui, PLAYERS_HEADING, Some(&note));
    ui.add_space(HEADING_TO_LIST_GAP_PX);
    let own = launcher.effective_player_name();
    let own_status = launcher.your_player_status();
    menu_row(
        ui,
        &format!("{own}  (you)"),
        MenuValue::Capitals(own_status.text()),
        true,
        true,
    );
    for member in &launcher.link.members {
        let ready = member.lobby.is_some_and(|lobby| lobby.ready);
        let status = if ready { READY_TEXT } else { NOT_READY_TEXT };
        menu_row(ui, &member.name, MenuValue::Capitals(status), false, true);
    }
}

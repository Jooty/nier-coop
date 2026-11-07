use eframe::egui::Ui;
use rc_proto::PartnerBody;

use crate::app::{Launcher, NetForms};
use crate::lobby::body_choice_shown;
use crate::net::NetState;

const NIER_LABEL: &str = "Nier";
const PROLOGUE_CHARACTER_LABEL: &str = "The prologue character";

pub(super) fn draw_body_choice(
    ui: &mut Ui,
    launcher: &mut Launcher,
    forms: &NetForms,
    editable: bool,
) {
    let state = forms.net.status().state;
    let hosting = matches!(state, NetState::Hosting { .. });
    let connected = matches!(state, NetState::Connected { .. });
    if !body_choice_shown(hosting, connected, launcher.link.guest) {
        return;
    }
    ui.label("Your character in this co-op game");
    let mut body = launcher.config.body;
    ui.add_enabled_ui(editable, |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut body, PartnerBody::Nier, NIER_LABEL);
            ui.selectable_value(
                &mut body,
                PartnerBody::PrologueCharacter,
                PROLOGUE_CHARACTER_LABEL,
            );
        });
    });
    launcher.set_body(body);
}

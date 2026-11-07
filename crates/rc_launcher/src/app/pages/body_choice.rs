use eframe::egui::Ui;
use rc_proto::PartnerBody;

use crate::app::{Launcher, NetForms};
use crate::lobby::{body_choice_shown, body_label};
use crate::net::NetState;

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
            ui.selectable_value(&mut body, PartnerBody::Nier, body_label(PartnerBody::Nier));
            ui.selectable_value(
                &mut body,
                PartnerBody::PrologueCharacter,
                body_label(PartnerBody::PrologueCharacter),
            );
            ui.selectable_value(
                &mut body,
                PartnerBody::SeedKaineE,
                body_label(PartnerBody::SeedKaineE),
            );
        });
    });
    launcher.set_body(body);
}

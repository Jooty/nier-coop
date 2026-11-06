use eframe::egui::{RichText, Ui};
use rc_proto::PartnerBody;

use crate::app::Launcher;

const NIER_LABEL: &str = "Nier";
const PROLOGUE_CHARACTER_LABEL: &str = "The prologue character";

pub(super) fn draw_body_choice(ui: &mut Ui, launcher: &mut Launcher, editable: bool) {
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
    ui.label(RichText::new("Remembered for next time. Your partner sees this body.").weak());
}

use eframe::egui::{self, Ui};

use crate::app::pages::body_choice::draw_body_choice;
use crate::app::pages::lobby::{claim_mode, draw_lobby, other_page_note, session_open};
use crate::app::style::{BAD, WARN};
use crate::app::{Launcher, NetForms};
use crate::lobby::NameError;
use crate::net::LOBBY_MODE_NEW_GAME;

pub(super) fn draw_new_game(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if !claim_mode(launcher, forms, LOBBY_MODE_NEW_GAME) {
        other_page_note(ui, launcher);
        return;
    }
    ui.label("Your name in this co-op game");
    let editable = !session_open(forms);
    ui.add_enabled(
        editable,
        egui::TextEdit::singleline(&mut launcher.new_game_name).desired_width(f32::INFINITY),
    );
    match launcher.name_check() {
        Ok(_) => {}
        Err(NameError::Empty) => {
            ui.colored_label(WARN, NameError::Empty.to_string());
        }
        Err(error) => {
            ui.colored_label(BAD, error.to_string());
        }
    }
    draw_body_choice(ui, launcher, editable);
    ui.separator();
    draw_lobby(ui, launcher, forms);
}

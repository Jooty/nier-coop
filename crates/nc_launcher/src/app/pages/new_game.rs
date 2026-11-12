use eframe::egui::Ui;

use crate::app::pages::campaign_lobby::draw_campaign_lobby;
use crate::app::pages::lobby::{claim_mode, other_page_note};
use crate::app::{Launcher, NetForms};
use crate::net::LOBBY_MODE_NEW_GAME;

pub(super) fn draw_new_game(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if !claim_mode(launcher, forms, LOBBY_MODE_NEW_GAME) {
        other_page_note(ui, launcher);
        return;
    }
    draw_campaign_lobby(ui, launcher, forms);
}

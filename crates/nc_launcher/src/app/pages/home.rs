use eframe::egui::{self, RichText, Ui};

use crate::app::pages::lobby::session_open;
use crate::app::pages::tone::tone_color;
use crate::app::{Launcher, NetForms, heading_text};
use crate::home::{GameCard, card_view};

const CARD_SPACING: f32 = 12.0;

pub(super) fn draw_home(ui: &mut Ui, launcher: &mut Launcher, forms: &NetForms) {
    let switch_locked = !launcher.can_switch_game() || session_open(forms);
    let cards = launcher.cards.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for card in &cards {
                draw_card(ui, launcher, card, switch_locked);
                ui.add_space(CARD_SPACING);
            }
        });
}

fn draw_card(ui: &mut Ui, launcher: &mut Launcher, card: &GameCard, switch_locked: bool) {
    let view = card_view(card);
    let is_active = card.profile.game_id == launcher.profile.game_id;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.add_enabled_ui(!view.greyed_out, |ui| {
            ui.label(heading_text(view.title));
            ui.colored_label(tone_color(view.found_tone), &view.found_text);
            ui.colored_label(tone_color(view.mod_tone), &view.mod_text);
            let blocked = !is_active && switch_locked;
            let mut play = ui.add_enabled(view.play_enabled && !blocked, egui::Button::new("Play"));
            if blocked {
                play = play.on_disabled_hover_text("Finish the current game first.");
            }
            if play.clicked() {
                launcher.open_game(card.profile);
            }
        });
        if view.greyed_out {
            ui.label(RichText::new("Install the game to play it here.").weak());
        }
    });
}

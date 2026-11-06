use eframe::egui::{self, Ui};

use crate::app::pages::continue_game::draw_continue;
use crate::app::pages::footer::draw_footer;
use crate::app::pages::new_game::draw_new_game;
use crate::app::pages::settings::draw_settings;
use crate::app::{Launcher, NetForms};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    NewGame,
    Continue,
    Settings,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 3] = [
        (Tab::NewGame, "New game"),
        (Tab::Continue, "Continue"),
        (Tab::Settings, "Settings"),
    ];
}

pub fn draw(ui: &mut Ui, tab: &mut Tab, launcher: &mut Launcher, forms: &mut NetForms) {
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (value, label) in Tab::ALL {
                ui.selectable_value(tab, value, label);
            }
        });
    });
    egui::Panel::bottom("footer").show(ui, |ui| {
        draw_footer(ui, launcher, forms);
    });
    egui::CentralPanel::default().show(ui, |ui| match *tab {
        Tab::NewGame => draw_new_game(ui, launcher, forms),
        Tab::Continue => draw_continue(ui, launcher, forms),
        Tab::Settings => draw_settings(ui, launcher, forms),
    });
}

use eframe::egui::{self, Ui};

use crate::app::heading_text;
use crate::app::pages::continue_game::draw_continue;
use crate::app::pages::footer::draw_footer;
use crate::app::pages::home::draw_home;
use crate::app::pages::logs::draw_logs;
use crate::app::pages::new_game::draw_new_game;
use crate::app::pages::settings::draw_settings;
use crate::app::{Launcher, NetForms, RESTART_TAB_LOGS, RESTART_TAB_SETTINGS, Screen};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    NewGame,
    Continue,
    Settings,
    Logs,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 4] = [
        (Tab::NewGame, "New game"),
        (Tab::Continue, "Continue"),
        (Tab::Settings, "Settings"),
        (Tab::Logs, "Logs"),
    ];

    pub fn from_restart_value(value: Option<&str>) -> Tab {
        match value {
            Some(RESTART_TAB_SETTINGS) => Tab::Settings,
            Some(RESTART_TAB_LOGS) => Tab::Logs,
            _ => Tab::NewGame,
        }
    }
}

pub fn draw(ui: &mut Ui, tab: &mut Tab, launcher: &mut Launcher, forms: &mut NetForms) {
    egui::Panel::bottom("footer").show(ui, |ui| {
        draw_footer(ui, launcher, forms);
    });
    if launcher.screen == Screen::Home {
        egui::CentralPanel::default().show(ui, |ui| draw_home(ui, launcher, forms));
        return;
    }
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            if ui.button(heading_text("Home")).clicked() {
                launcher.go_home();
            }
            ui.label(heading_text(launcher.profile.display_name));
            ui.separator();
            for (value, label) in Tab::ALL {
                ui.selectable_value(tab, value, heading_text(label));
            }
        });
    });
    egui::CentralPanel::default().show(ui, |ui| match *tab {
        Tab::NewGame => draw_new_game(ui, launcher, forms),
        Tab::Continue => draw_continue(ui, launcher, forms),
        Tab::Settings => draw_settings(ui, launcher, forms),
        Tab::Logs => draw_logs(ui, launcher),
    });
}

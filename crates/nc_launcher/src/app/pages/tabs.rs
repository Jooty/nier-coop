use eframe::egui::{self, Frame, Margin, Ui};

use crate::app::game_look::neutral_look;
use crate::app::pages::continue_game::draw_continue;
use crate::app::pages::footer::{STATUS_LINE_HEIGHT_PX, draw_footer};
use crate::app::pages::home::draw_home;
use crate::app::pages::logs::draw_logs;
use crate::app::pages::new_game::draw_new_game;
use crate::app::pages::settings::draw_settings;
use crate::app::pages::top_bar::{TOP_BAR_HEIGHT_PX, draw_top_bar};
use crate::app::palette::BACKGROUND;
use crate::app::{
    Launcher, NetForms, RESTART_TAB_LOGS, RESTART_TAB_SETTINGS, Screen, apply_game_accent,
    game_look,
};

const BAR_SIDE_PADDING_PX: i8 = 24;
const PAGE_TOP_PADDING_PX: i8 = 18;
const PAGE_BOTTOM_PADDING_PX: i8 = 16;

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
    let look = match launcher.screen {
        Screen::Home => neutral_look(),
        Screen::Game => game_look(launcher.profile),
    };
    apply_game_accent(ui.visuals_mut(), look);
    egui::Panel::bottom("footer")
        .exact_size(STATUS_LINE_HEIGHT_PX)
        .frame(bar_frame())
        .show(ui, |ui| {
            draw_footer(ui, launcher, forms);
        });
    if launcher.screen == Screen::Game {
        egui::Panel::top("tabs")
            .exact_size(TOP_BAR_HEIGHT_PX)
            .frame(bar_frame())
            .show_separator_line(false)
            .show(ui, |ui| draw_top_bar(ui, tab, launcher, look));
    }
    egui::CentralPanel::default()
        .frame(page_frame())
        .show(ui, |ui| match (launcher.screen, *tab) {
            (Screen::Home, _) => draw_home(ui, launcher, forms),
            (Screen::Game, Tab::NewGame) => draw_new_game(ui, launcher, forms),
            (Screen::Game, Tab::Continue) => draw_continue(ui, launcher, forms),
            (Screen::Game, Tab::Settings) => draw_settings(ui, launcher, forms),
            (Screen::Game, Tab::Logs) => draw_logs(ui, launcher),
        });
}

fn bar_frame() -> Frame {
    Frame::NONE
        .fill(BACKGROUND)
        .inner_margin(Margin::symmetric(BAR_SIDE_PADDING_PX, 0))
}

fn page_frame() -> Frame {
    Frame::NONE.fill(BACKGROUND).inner_margin(Margin {
        left: BAR_SIDE_PADDING_PX,
        right: BAR_SIDE_PADDING_PX,
        top: PAGE_TOP_PADDING_PX,
        bottom: PAGE_BOTTOM_PADDING_PX,
    })
}

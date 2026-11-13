use eframe::egui::{self, Frame, Margin, Rect, Ui, UiBuilder, pos2};

use crate::app::motion::{mark_page, set_motion};
use crate::app::pages::continue_game::draw_continue;
use crate::app::pages::home::draw_home;
use crate::app::pages::lobby::session_open;
use crate::app::pages::logs::draw_logs;
use crate::app::pages::new_game::draw_new_game;
use crate::app::pages::settings::draw_settings;
use crate::app::pages::top_bar::{TOP_BAR_HEIGHT_PX, draw_home_bar, draw_top_bar};
use crate::app::palette::GROUND;
use crate::app::parts::{DOUBLE_RULE_HEIGHT_PX, paint_double_rule, paint_grid_ground};
use crate::app::{Launcher, NetForms, RESTART_TAB_LOGS, RESTART_TAB_SETTINGS, Screen};
use crate::net::LOBBY_MODE_CONTINUE;

pub(super) const SIDE_GUTTER_PX: f32 = 24.0;
const PAGE_TOP_GAP_PX: f32 = 16.0;
const PAGE_BOTTOM_GAP_PX: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    follow_the_lobby_tab(tab, launcher);
    set_motion(ui.ctx(), launcher.config.motion);
    let page = (
        launcher.screen,
        *tab,
        launcher.profile.game_id,
        session_open(forms),
    );
    mark_page(ui, page);
    egui::Panel::top("top-bar")
        .exact_size(TOP_BAR_HEIGHT_PX)
        .frame(bar_frame())
        .show_separator_line(false)
        .show(ui, |ui| match launcher.screen {
            Screen::Home => draw_home_bar(ui),
            Screen::Game => draw_top_bar(ui, tab, launcher),
        });
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(GROUND))
        .show(ui, |ui| {
            let whole = ui.max_rect();
            let painter = ui.painter();
            paint_double_rule(
                painter,
                whole.left() + SIDE_GUTTER_PX..=whole.right() - SIDE_GUTTER_PX,
                whole.top(),
            );
            let ground = Rect::from_min_max(
                pos2(whole.left(), whole.top() + DOUBLE_RULE_HEIGHT_PX),
                whole.right_bottom(),
            );
            paint_grid_ground(painter, ground);
            let page = Rect::from_min_max(
                pos2(
                    ground.left() + SIDE_GUTTER_PX,
                    ground.top() + PAGE_TOP_GAP_PX,
                ),
                pos2(
                    ground.right() - SIDE_GUTTER_PX,
                    ground.bottom() - PAGE_BOTTOM_GAP_PX,
                ),
            );
            ui.scope_builder(UiBuilder::new().max_rect(page), |ui| {
                match (launcher.screen, *tab) {
                    (Screen::Home, _) => draw_home(ui, tab, launcher, forms),
                    (Screen::Game, Tab::NewGame) => draw_new_game(ui, launcher, forms),
                    (Screen::Game, Tab::Continue) => draw_continue(ui, launcher, forms),
                    (Screen::Game, Tab::Settings) => draw_settings(ui, launcher, forms),
                    (Screen::Game, Tab::Logs) => draw_logs(ui, launcher),
                }
            });
        });
}

fn follow_the_lobby_tab(tab: &mut Tab, launcher: &mut Launcher) {
    if !matches!(*tab, Tab::NewGame | Tab::Continue) {
        return;
    }
    let Some(kind) = launcher.tab_to_follow.take() else {
        return;
    };
    *tab = if kind == LOBBY_MODE_CONTINUE {
        Tab::Continue
    } else {
        Tab::NewGame
    };
}

fn bar_frame() -> Frame {
    Frame::NONE
        .fill(GROUND)
        .inner_margin(Margin::symmetric(SIDE_GUTTER_PX as i8, 0))
}

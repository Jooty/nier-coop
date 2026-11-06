use std::sync::Arc;

use eframe::egui::{self, Theme, ThemePreference};

use crate::app::{
    Launcher, NetForms, Tab, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH, draw, imgui_dark_visuals,
};
use crate::net::StubNet;

struct LauncherApp {
    launcher: Launcher,
    forms: NetForms,
    tab: Tab,
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = ui.input(|input| input.time);
        self.launcher.tick(now, ui.ctx());
        self.forms.poll();
        draw(ui, &mut self.tab, &mut self.launcher, &mut self.forms);
    }
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_WIDTH, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|creation| {
            creation.egui_ctx.set_theme(ThemePreference::Dark);
            creation
                .egui_ctx
                .set_visuals_of(Theme::Dark, imgui_dark_visuals());
            Ok(Box::new(LauncherApp {
                launcher: Launcher::load(),
                forms: NetForms::new(Arc::new(StubNet)),
                tab: Tab::Play,
            }))
        }),
    )
}

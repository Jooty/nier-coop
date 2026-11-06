use std::sync::Arc;

use eframe::egui::{self, Theme, ThemePreference};

use crate::app::{
    Launcher, NetForms, Tab, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH, draw, imgui_dark_visuals,
};
use crate::net::{LauncherCore, LauncherNet, StubNet};
use rc_launcher_core::{LauncherConfig as CoreConfig, iroh_factory};
use rc_net::IrohConfig;

const EVENT_REPAINT_MILLIS: u64 = 250;

struct LauncherApp {
    launcher: Launcher,
    forms: NetForms,
    core: Option<Arc<LauncherCore>>,
    tab: Tab,
}

impl LauncherApp {
    fn start() -> Self {
        let mut launcher = Launcher::load();
        let config = CoreConfig::new(&launcher.player_name());
        let core = match LauncherCore::new(config, iroh_factory(IrohConfig::default())) {
            Ok(core) => Some(Arc::new(core)),
            Err(error) => {
                launcher
                    .log
                    .push(format!("networking could not start: {error}"));
                None
            }
        };
        launcher.link_port = core.as_ref().map(|core| core.link_port());
        let net: Arc<dyn LauncherNet> = match &core {
            Some(core) => core.clone(),
            None => Arc::new(StubNet),
        };
        Self {
            launcher,
            forms: NetForms::new(net),
            core,
            tab: Tab::Play,
        }
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = ui.input(|input| input.time);
        self.launcher.tick(now, ui.ctx());
        self.forms.poll();
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(EVENT_REPAINT_MILLIS));
        if let Some(core) = &self.core {
            let events = core.drain_events();
            self.launcher.note_events(&events);
        }
        draw(ui, &mut self.tab, &mut self.launcher, &mut self.forms);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.launcher.release_game_link();
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
            Ok(Box::new(LauncherApp::start()))
        }),
    )
}

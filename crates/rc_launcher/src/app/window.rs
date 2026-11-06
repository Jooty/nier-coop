use std::sync::Arc;

use eframe::egui::{self, Theme, ThemePreference};

use crate::app::{
    Launcher, NetForms, Role, Tab, WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH, apply_shared_spacing,
    draw, imgui_dark_visuals,
};
use crate::net::{LauncherCore, LauncherNet, StubNet};
use rc_launcher_core::{LauncherConfig as CoreConfig, iroh_factory};
use rc_net::IrohConfig;

const APP_ID: &str = "replicant-coop";
const EVENT_REPAINT_MILLIS: u64 = 250;

struct LauncherApp {
    launcher: Launcher,
    forms: NetForms,
    core: Option<Arc<LauncherCore>>,
    tab: Tab,
    name_sent_to_core: String,
}

impl LauncherApp {
    fn start() -> Self {
        let mut launcher = Launcher::load();
        let config = CoreConfig::new(&launcher.effective_player_name());
        let core = match LauncherCore::new(config, iroh_factory(IrohConfig::default())) {
            Ok(core) => Some(Arc::new(core)),
            Err(error) => {
                launcher
                    .log
                    .push(format!("networking could not start: {error}"));
                None
            }
        };
        let name_sent_to_core = launcher.effective_player_name();
        let session_options = launcher.config.session_options();
        if let Some(core) = &core {
            core.set_player_settings(launcher.player_settings());
            core.set_session_options(session_options);
        }
        launcher.link_port = core.as_ref().map(|core| core.link_port());
        let net: Arc<dyn LauncherNet> = match &core {
            Some(core) => core.clone(),
            None => Arc::new(StubNet),
        };
        Self {
            launcher,
            forms: {
                let mut forms = NetForms::new(net);
                forms.session_options = session_options;
                forms
            },
            core,
            tab: Tab::Play,
            name_sent_to_core,
        }
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = ui.input(|input| input.time);
        self.launcher.tick(now, ui.ctx());
        if self.forms.poll(ui.ctx()) {
            self.launcher.launch(Role::Guest);
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(EVENT_REPAINT_MILLIS));
        if let Some(core) = &self.core {
            let name = self.launcher.effective_player_name();
            if name != self.name_sent_to_core {
                core.set_player_name(&name);
                self.name_sent_to_core = name;
            }
            let events = core.drain_events();
            self.launcher.note_events(&events);
            if !self.launcher.link.guest {
                self.forms.session_options = self.launcher.config.session_options();
            }
            self.launcher.link = core.link_status();
        }
        draw(ui, &mut self.tab, &mut self.launcher, &mut self.forms);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(core) = &self.core {
            core.shutdown();
        }
        self.launcher.release_game_link();
    }
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_WIDTH, 300.0])
            .with_app_id(APP_ID)
            .with_icon(window_icon()),
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
            creation.egui_ctx.all_styles_mut(apply_shared_spacing);
            Ok(Box::new(LauncherApp::start()))
        }),
    )
}

fn window_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon.png")).unwrap_or_default()
}

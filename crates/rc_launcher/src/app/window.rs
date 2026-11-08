use std::sync::Arc;

use eframe::egui::{self, Theme, ThemePreference};

use crate::app::{
    Launcher, NetForms, RESTART_TAB_ARGUMENT, RESTART_UPDATED_ARGUMENT, Tab, UpdateState,
    WINDOW_HEIGHT, WINDOW_TITLE, WINDOW_WIDTH, apply_shared_spacing, draw, imgui_dark_visuals,
    install_heading_font,
};
use crate::net::{LauncherCore, LauncherNet, LobbyReady, StubNet};
use rc_launcher_core::{LauncherConfig as CoreConfig, iroh_factory};
use rc_net::IrohConfig;
use rc_proto::PartnerBody;

const APP_ID: &str = "replicant-coop";
const EVENT_REPAINT_MILLIS: u64 = 250;

struct LauncherApp {
    launcher: Launcher,
    forms: NetForms,
    core: Option<Arc<LauncherCore>>,
    tab: Tab,
    name_sent_to_core: String,
    lobby_sent: Option<LobbyReady>,
    body_sent: Option<PartnerBody>,
}

impl LauncherApp {
    fn start() -> Self {
        let mut launcher = Launcher::load();
        let (tab, updated_to) = restart_arguments();
        if let Some(short) = updated_to {
            launcher.updater.state = UpdateState::Updated(short);
        }
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
            tab,
            name_sent_to_core,
            lobby_sent: None,
            body_sent: None,
        }
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = ui.input(|input| input.time);
        self.launcher.tick(now, ui.ctx());
        self.forms.poll(ui.ctx());
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
        let ready = self.launcher.own_lobby_ready();
        if self.lobby_sent != Some(ready) {
            self.forms.net.send_lobby_ready(ready);
            self.lobby_sent = Some(ready);
        }
        if self.launcher.link.guest {
            let body = self.launcher.own_lobby_body();
            if self.body_sent != Some(body) {
                self.forms.net.send_lobby_body(body);
                self.body_sent = Some(body);
            }
        }
        if self.launcher.take_quit_request() {
            self.forms.net.ask_game_to_quit();
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

fn restart_arguments() -> (Tab, Option<String>) {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let value_of = |name: &str| {
        arguments
            .iter()
            .position(|argument| argument == name)
            .and_then(|index| arguments.get(index + 1))
            .cloned()
    };
    let tab = Tab::from_restart_value(value_of(RESTART_TAB_ARGUMENT).as_deref());
    (tab, value_of(RESTART_UPDATED_ARGUMENT))
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
            install_heading_font(&creation.egui_ctx);
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

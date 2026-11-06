use std::path::PathBuf;

use eframe::egui::Context;

use crate::app::actions::ledger_path;
use crate::app::describe_event;
use crate::app::{
    ActionReport, GAME_POLL_SECONDS, STEAM_RUN_URL_PREFIX, Task, run_install, run_uninstall,
};
use crate::config::{LauncherConfig, cache_dir, config_path};
use crate::game_link::{clear_link, write_link};
use crate::install::{
    Ledger, ModStatus, dll_next_to_executable, game_running, mod_status, newest_cached_ci_dll,
    open_url,
};
use crate::net::{DEFAULT_PLAYER_NAME, LauncherEvent};
use crate::saves::{backup_root, backup_saves, format_stamp, save_dir};
use crate::steam::{APP_ID, GameInstall, detect_game};

pub struct Launcher {
    pub config: LauncherConfig,
    pub game: Option<GameInstall>,
    pub dll_source: Option<PathBuf>,
    pub mod_status: ModStatus,
    pub game_running: bool,
    pub log: Vec<String>,
    pub override_text: String,
    pub name_text: String,
    pub link_port: Option<u16>,
    pub work: Option<Task<ActionReport>>,
    last_game_poll: f64,
}

impl Launcher {
    pub fn load() -> Self {
        let config = config_path()
            .map(|path| LauncherConfig::load(&path))
            .unwrap_or_default();
        let override_text = config
            .game_dir
            .as_ref()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default();
        let name_text = config.player_name.clone().unwrap_or_default();
        let mut launcher = Self {
            config,
            game: None,
            dll_source: None,
            mod_status: ModStatus::NotInstalled,
            game_running: false,
            log: Vec::new(),
            override_text,
            name_text,
            link_port: None,
            work: None,
            last_game_poll: f64::NEG_INFINITY,
        };
        launcher.refresh();
        launcher
    }

    pub fn refresh(&mut self) {
        self.game = detect_game(&self.config);
        self.dll_source = dll_next_to_executable()
            .or_else(|| cache_dir().and_then(|dir| newest_cached_ci_dll(&dir)));
        self.mod_status = match (&self.game, ledger_path()) {
            (Some(game), Some(ledger)) => mod_status(
                &game.game_dir,
                self.dll_source.as_deref(),
                &Ledger::new(&ledger),
            ),
            _ => ModStatus::NotInstalled,
        };
        self.game_running = game_running();
    }

    pub fn tick(&mut self, now_seconds: f64, ctx: &Context) {
        if let Some(report) = self.work.as_ref().and_then(Task::try_take) {
            self.work = None;
            if let Some(path) = report.fetched_dll.clone() {
                self.dll_source = Some(path);
            }
            self.log.extend(report.lines);
            self.refresh();
        }
        if now_seconds - self.last_game_poll >= GAME_POLL_SECONDS {
            self.last_game_poll = now_seconds;
            self.game_running = game_running();
        }
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(GAME_POLL_SECONDS));
    }

    pub fn busy(&self) -> bool {
        self.work.is_some()
    }

    pub fn start_install(&mut self, ctx: &Context) {
        let Some(game) = self.game.clone() else {
            return;
        };
        let source = self.dll_source.clone();
        self.log.push("installing...".to_owned());
        self.work = Some(Task::spawn(move || run_install(game, source), ctx.clone()));
    }

    pub fn start_uninstall(&mut self, ctx: &Context) {
        let Some(game) = self.game.clone() else {
            return;
        };
        self.log.push("uninstalling...".to_owned());
        self.work = Some(Task::spawn(move || run_uninstall(game), ctx.clone()));
    }

    pub fn player_name(&self) -> String {
        self.config
            .player_name
            .clone()
            .unwrap_or_else(|| DEFAULT_PLAYER_NAME.to_owned())
    }

    pub fn apply_name(&mut self) {
        let text = self.name_text.trim();
        self.config.player_name = (!text.is_empty()).then(|| text.to_owned());
        self.save_config();
    }

    pub fn note_events(&mut self, events: &[LauncherEvent]) {
        self.log.extend(events.iter().map(describe_event));
    }

    pub fn release_game_link(&mut self) {
        if let Some(game) = &self.game
            && let Err(error) = clear_link(&game.game_dir)
        {
            self.log
                .push(format!("could not clean up the link: {error}"));
        }
    }

    pub fn launch(&mut self) {
        if self.game_running {
            self.log.push("the game is already running".to_owned());
            return;
        }
        if let (Some(game), Some(port)) = (&self.game, self.link_port) {
            let name = self.player_name();
            if let Err(error) = write_link(&game.game_dir, port, &name) {
                self.log
                    .push(format!("could not write the link for the game: {error}"));
            }
        }
        match open_url(&format!("{STEAM_RUN_URL_PREFIX}{APP_ID}")) {
            Ok(()) => self.log.push("asked Steam to start the game".to_owned()),
            Err(error) => self.log.push(format!("could not open Steam: {error}")),
        }
    }

    pub fn backup_saves(&mut self) {
        let (Some(game), Some(root)) = (&self.game, backup_root()) else {
            self.log
                .push("no game found, so no saves to back up".to_owned());
            return;
        };
        let Some(saves) = save_dir(game) else {
            self.log
                .push("could not find the Documents folder".to_owned());
            return;
        };
        let stamp = format_stamp(&chrono::Local::now());
        match backup_saves(&saves, &root, &stamp) {
            Ok((_, files)) => self
                .log
                .push(format!("saved backup {stamp} ({files} files)")),
            Err(error) => self.log.push(format!("backup failed: {error}")),
        }
    }

    pub fn apply_override(&mut self) {
        let text = self.override_text.trim();
        self.config.game_dir = (!text.is_empty()).then(|| PathBuf::from(text));
        self.save_config();
        self.refresh();
    }

    pub fn clear_override(&mut self) {
        self.override_text.clear();
        self.apply_override();
    }

    fn save_config(&mut self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Err(error) = self.config.save(&path) {
            self.log.push(format!("could not save settings: {error}"));
        }
    }
}

use std::path::PathBuf;

use eframe::egui::Context;

use rc_proto::{GameReadiness, PartnerBody};

use crate::app::actions::ledger_path;
use crate::app::describe_event;
use crate::app::{
    ActionReport, GAME_POLL_SECONDS, LogUploader, STEAM_RUN_URL_PREFIX, Task, Updater, run_install,
    run_uninstall,
};
use crate::config::data_dir;
use crate::config::{LauncherConfig, cache_dir, config_path};
use crate::game_link::{
    clear_boot, clear_link, write_boot, write_dev_switches, write_link, write_log, write_new_game,
    write_settings,
};
use crate::install::{
    GAME_NEVER_APPEARED_SECONDS, GameWatch, GameWatchStep, Ledger, ModStatus,
    dll_next_to_executable, game_pids, game_running, mod_mismatch, mod_status, newer_dll,
    newest_cached_ci_dll, open_url,
};
use crate::lobby::{
    GUEST_WAIT_FOR_HOST_GAME_SECONDS, GameState, GuestWaitStep, GuestWaiting, LaunchMode,
    NameError, OwnGameInputs, ReadyInputs, guest_accepts_start, lobby_ready, new_game_slot,
    own_game_state, partner_game_state, resolve_mode, validate_name,
};
use crate::logs::{
    LAUNCHER_LOG_BEFORE_ROLE, LogWriter, game_log_name, launcher_log_name, logs_dir,
    path_as_the_game_sees_it, rotate_log,
};
use crate::net::{
    ConnectionState, DEFAULT_PLAYER_NAME, LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LauncherEvent,
    LinkStatus, LobbyReady, PlayerSettings, SessionOptions,
};
use crate::saves::{
    SlotBodies, SlotError, SlotSummary, backup_coop_file, backup_root, backup_saves,
    coop_backup_root, coop_bodies_path, coop_copy_target_slot, coop_gamedata_path,
    copy_slot_into_coop, default_slot, format_stamp, fresh_coop_file, gamedata_path, list_backups,
    read_coop_slots, read_slot_bodies, read_slots, restore_coop_backup, save_dir, slot_body,
    slot_player_name, store_slot_body,
};
use crate::second_copy::{
    EARLY_EXIT_SECONDS, LOGS_FOLDER_NAME, SecondCopy, build_command, start_second_copy,
};
use crate::steam::{APP_ID, GameInstall, detect_game};

const MAX_LOG_LINES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Host,
    Guest,
}

struct ResolvedLaunch {
    name: String,
    boot_slot: Option<u8>,
    new_game: Option<(u8, String)>,
    body: PartnerBody,
}

pub struct Launcher {
    pub config: LauncherConfig,
    pub game: Option<GameInstall>,
    pub dll_source: Option<PathBuf>,
    pub mod_status: ModStatus,
    pub game_running: bool,
    pub log: Vec<String>,
    pub play_error: Option<String>,
    pub export_note: Option<String>,
    second_copy: Option<SecondCopy>,
    pub override_text: String,
    pub new_game_name: String,
    pub lobby_mode: u8,
    pub lobby_started: bool,
    pub link_port: Option<u16>,
    pub work: Option<Task<ActionReport>>,
    pub updater: Updater,
    pub log_uploader: LogUploader,
    pub link: LinkStatus,
    pub slots: Vec<SlotSummary>,
    pub slot_bodies: SlotBodies,
    pub normal_slots: Vec<SlotSummary>,
    pub copy_from_slot: Option<u8>,
    pub selected_slot: Option<u8>,
    pub host_options_seen: Option<SessionOptions>,
    guest_waiting: Option<GuestWaiting>,
    game_watch: Option<GameWatch>,
    own_readiness: GameReadiness,
    game_closed: bool,
    game_linked_since_launch: bool,
    partner_game_seen_linked: bool,
    pub backups: Vec<String>,
    pub selected_backup: Option<String>,
    log_writer: Option<LogWriter>,
    log_written: usize,
    role_log_chosen: bool,
    last_game_poll: f64,
}

impl Launcher {
    pub fn load() -> Self {
        let launcher_started_at = std::time::SystemTime::now();
        let config = config_path()
            .map(|path| LauncherConfig::load(&path))
            .unwrap_or_default();
        let override_text = config
            .game_dir
            .as_ref()
            .map(|dir| dir.display().to_string())
            .unwrap_or_default();
        let new_game_name = config.player_name.clone().unwrap_or_default();
        let mut launcher = Self {
            config,
            game: None,
            dll_source: None,
            mod_status: ModStatus::NotInstalled,
            game_running: false,
            log: Vec::new(),
            play_error: None,
            export_note: None,
            second_copy: None,
            override_text,
            new_game_name,
            lobby_mode: LOBBY_MODE_NEW_GAME,
            lobby_started: false,
            link_port: None,
            work: None,
            updater: Updater::default(),
            log_uploader: LogUploader::new(launcher_started_at),
            link: LinkStatus::default(),
            slots: Vec::new(),
            slot_bodies: SlotBodies::new(),
            normal_slots: Vec::new(),
            copy_from_slot: None,
            selected_slot: None,
            host_options_seen: None,
            guest_waiting: None,
            game_watch: None,
            own_readiness: GameReadiness::NotLinked,
            game_closed: false,
            game_linked_since_launch: false,
            partner_game_seen_linked: false,
            backups: Vec::new(),
            selected_backup: None,
            log_writer: logs_dir()
                .and_then(|dir| LogWriter::start(&dir, LAUNCHER_LOG_BEFORE_ROLE).ok()),
            log_written: 0,
            role_log_chosen: false,
            last_game_poll: f64::NEG_INFINITY,
        };
        launcher.refresh();
        launcher
    }

    pub fn refresh(&mut self) {
        self.game = detect_game(&self.config);
        self.dll_source = newer_dll(
            dll_next_to_executable(),
            cache_dir().and_then(|dir| newest_cached_ci_dll(&dir)),
        );
        self.mod_status = match (&self.game, ledger_path()) {
            (Some(game), Some(ledger)) => mod_status(
                &game.game_dir,
                self.dll_source.as_deref(),
                &Ledger::new(&ledger),
            ),
            _ => ModStatus::NotInstalled,
        };
        self.game_running = game_running();
        self.refresh_saves();
    }

    pub fn refresh_saves(&mut self) {
        self.backups = coop_backup_root()
            .map(|root| list_backups(&root))
            .unwrap_or_default();
        self.normal_slots = self
            .game
            .as_ref()
            .and_then(save_dir)
            .and_then(|dir| read_slots(&dir).ok())
            .unwrap_or_default();
        if !self.copy_from_slot.is_some_and(|picked| {
            self.normal_slots
                .iter()
                .any(|slot| slot.index == picked && slot.used)
        }) {
            self.copy_from_slot = default_slot(&self.normal_slots);
        }
        self.slots = match self
            .game
            .as_ref()
            .and_then(save_dir)
            .map(|dir| read_coop_slots(&dir))
        {
            Some(Ok(slots)) => slots,
            Some(Err(SlotError::Io(error))) if error.kind() == std::io::ErrorKind::NotFound => {
                Vec::new()
            }
            Some(Err(error)) => {
                self.log
                    .push(format!("could not read the save slots: {error}"));
                Vec::new()
            }
            None => Vec::new(),
        };
        self.slot_bodies = self
            .game
            .as_ref()
            .and_then(save_dir)
            .and_then(|dir| coop_bodies_path(&dir))
            .map(|path| read_slot_bodies(&path))
            .unwrap_or_default();
        let still_valid = self.selected_slot.is_some_and(|picked| {
            self.slots
                .iter()
                .any(|slot| slot.index == picked && slot.used)
        });
        if !still_valid {
            self.selected_slot = default_slot(&self.slots);
        }
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
        self.tick_updates(ctx);
        self.tick_log_upload(ctx);
        self.poll_second_copy();
        self.write_new_log_lines();
        if self.log.len() > MAX_LOG_LINES {
            let dropped = self.log.len() - MAX_LOG_LINES;
            self.log.drain(..dropped);
            self.log_written = self.log_written.saturating_sub(dropped);
        }
        if now_seconds - self.last_game_poll >= GAME_POLL_SECONDS {
            self.last_game_poll = now_seconds;
            self.poll_game_process(now_seconds, ctx);
        }
        self.step_guest_wait(now_seconds);
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

    pub fn set_body(&mut self, body: PartnerBody) {
        if self.config.body != body {
            self.config.body = body;
            self.save_config();
        }
    }

    pub fn player_name(&self) -> String {
        self.config
            .player_name
            .clone()
            .unwrap_or_else(|| DEFAULT_PLAYER_NAME.to_owned())
    }

    pub fn effective_player_name(&self) -> String {
        let typed = validate_name(&self.new_game_name).ok();
        let chosen = if self.lobby_mode == LOBBY_MODE_CONTINUE {
            slot_player_name(&self.slots, self.selected_slot).or(typed)
        } else {
            typed
        };
        chosen.unwrap_or_else(|| self.player_name())
    }

    pub fn name_check(&self) -> Result<String, NameError> {
        validate_name(&self.new_game_name)
    }

    pub fn lobby_ready_as(&self, is_guest: bool) -> LobbyReady {
        lobby_ready(ReadyInputs {
            mode: self.lobby_mode,
            game_ok: self.can_start_game(),
            name_ok: self.name_check().is_ok(),
            new_game_slot_free: new_game_slot(&self.slots).is_some(),
            continue_slot: self.selected_slot,
            is_guest,
        })
    }

    pub fn own_lobby_ready(&self) -> LobbyReady {
        self.lobby_ready_as(self.link.guest)
    }

    pub fn lobby_block_reason(&self, is_guest: bool) -> Option<String> {
        if self.lobby_ready_as(is_guest).ready {
            return None;
        }
        if self.game.is_none() {
            return Some("Game not found. Set the folder in Settings.".to_owned());
        }
        if self.game_running && !crate::DEV_BUILD {
            return Some("The game is already running.".to_owned());
        }
        if !self.can_start_game() {
            return Some("No rc_hook.dll next to the launcher to install.".to_owned());
        }
        let continuing = self.lobby_mode == LOBBY_MODE_CONTINUE;
        if continuing && self.selected_slot.is_none() && !is_guest {
            return Some("Pick a save slot to continue.".to_owned());
        }
        if let Err(error) = self.name_check() {
            return Some(error.to_string());
        }
        Some("All six co-op slots are in use.".to_owned())
    }

    pub fn note_events(&mut self, events: &[LauncherEvent]) {
        self.log.extend(events.iter().map(describe_event));
        for event in events {
            match event {
                LauncherEvent::GameChangedSettings { settings } => {
                    self.config.take_player_settings(*settings);
                    self.apply_settings();
                }
                LauncherEvent::SessionOptionsChanged { options } if self.link.guest => {
                    self.host_options_seen = Some(*options);
                }
                LauncherEvent::SessionOptionsChanged { options } => {
                    self.config.take_session_options(*options);
                    self.apply_settings();
                }
                LauncherEvent::OwnGameReadiness { readiness } => self.own_readiness = *readiness,
                LauncherEvent::PartnerGameReadiness { readiness } => {
                    self.partner_game_seen_linked |= *readiness != GameReadiness::NotLinked;
                }
                LauncherEvent::GameLinked => self.game_linked_since_launch = true,
                LauncherEvent::GameUnlinked { .. }
                    if self.game_linked_since_launch && self.lobby_started =>
                {
                    self.mark_game_closed();
                }
                LauncherEvent::LobbyStartReceived { .. } => self.start_as_guest(),
                LauncherEvent::PeerLeft { .. }
                | LauncherEvent::Stopped
                | LauncherEvent::Failed { .. } => {
                    self.lobby_started = false;
                    self.guest_waiting = None;
                    self.partner_game_seen_linked = false;
                }
                _ => {}
            }
        }
    }

    pub fn release_game_link(&mut self) {
        if let Some(copy) = self.second_copy.take() {
            copy.kill_tree();
        }
        if let Some(game) = &self.game
            && let Err(error) = clear_link(&game.game_dir).and_then(|()| clear_boot(&game.game_dir))
        {
            self.log
                .push(format!("could not clean up the link: {error}"));
        }
    }

    pub fn can_start_game(&self) -> bool {
        self.game.is_some()
            && (!self.game_running || crate::DEV_BUILD)
            && !self.busy()
            && (self.mod_status == ModStatus::Current || self.dll_source.is_some())
    }

    fn start_as_guest(&mut self) {
        let own = self.own_lobby_ready();
        let connected = self.link.state == ConnectionState::Connected;
        if !guest_accepts_start(connected, self.link.guest, self.lobby_started, own) {
            self.log.push(
                "ignored a start from the host: not connected, not ready or already started"
                    .to_owned(),
            );
            return;
        }
        self.begin_game_launch();
        self.lobby_started = true;
        self.guest_waiting = Some(GuestWaiting::new(own.mode));
        self.log
            .push("waiting for the host's game to start".to_owned());
    }

    fn begin_game_launch(&mut self) {
        self.game_closed = false;
        self.game_linked_since_launch = false;
        self.partner_game_seen_linked = self.link.peer_readiness != GameReadiness::NotLinked;
    }

    fn step_guest_wait(&mut self, now_seconds: f64) {
        let Some(waiting) = self.guest_waiting.as_mut() else {
            return;
        };
        let mode = waiting.mode;
        match waiting.step(self.link.peer_readiness, now_seconds) {
            GuestWaitStep::KeepWaiting => {}
            GuestWaitStep::LaunchNow => {
                self.guest_waiting = None;
                self.lobby_started = false;
                self.launch(Role::Guest, mode);
            }
            GuestWaitStep::GiveUp => {
                self.guest_waiting = None;
                self.lobby_started = false;
                self.fail_launch(format!(
                    "the host's game did not start within {GUEST_WAIT_FOR_HOST_GAME_SECONDS:.0} s"
                ));
            }
        }
    }

    fn poll_game_process(&mut self, now_seconds: f64, ctx: &Context) {
        let pids = game_pids();
        self.game_running = !pids.is_empty();
        let Some(watch) = self.game_watch.as_mut() else {
            return;
        };
        match watch.step(&pids, now_seconds) {
            GameWatchStep::Waiting | GameWatchStep::Running => {}
            GameWatchStep::Exited => {
                self.game_watch = None;
                self.log.push("the game closed".to_owned());
                self.mark_game_closed();
                self.upload_after_game(ctx);
            }
            GameWatchStep::NeverAppeared => {
                self.game_watch = None;
                self.fail_launch(format!(
                    "the game did not start within {GAME_NEVER_APPEARED_SECONDS:.0} s"
                ));
                self.mark_game_closed();
            }
        }
    }

    fn mark_game_closed(&mut self) {
        self.game_watch = None;
        self.game_closed = true;
        self.lobby_started = false;
        self.own_readiness = GameReadiness::NotLinked;
    }

    pub fn your_game_state(&self) -> GameState {
        own_game_state(OwnGameInputs {
            started: self.lobby_started,
            closed: self.game_closed,
            waiting_for_host_game: self.guest_waiting.is_some(),
            own: self.own_readiness,
            partner: self.link.peer_readiness,
        })
    }

    pub fn their_game_state(&self) -> GameState {
        partner_game_state(self.link.peer_readiness, self.partner_game_seen_linked)
    }

    pub fn launch(&mut self, role: Role, mode: u8) -> bool {
        self.switch_launcher_log(role);
        self.note_run_started(role);
        self.refresh();
        if role == Role::Host {
            self.begin_game_launch();
        }
        let Some(resolved) = self.resolve_launch(mode) else {
            self.fail_launch(
                "could not start: no valid name or save slot for this mode".to_owned(),
            );
            return false;
        };
        self.play_error = None;
        self.lobby_started = true;
        if self.game_running {
            self.launch_second_copy(role, &resolved);
            return self.lobby_started;
        }
        if self.mod_status != ModStatus::Current
            && let Some(game) = self.game.clone()
        {
            let report = run_install(game, self.dll_source.clone());
            let failed = report.failed;
            self.log.extend(report.lines);
            self.refresh();
            if failed {
                self.lobby_started = false;
                return false;
            }
        }
        let log_path = self.start_game_log(role);
        if let (Some(game), Some(port)) = (&self.game, self.link_port) {
            if let Err(error) = write_link(&game.game_dir, port, &resolved.name) {
                self.log
                    .push(format!("could not write the link for the game: {error}"));
            }
            if let Some(log_path) = &log_path
                && let Err(error) = write_log(&game.game_dir, log_path)
            {
                self.log
                    .push(format!("could not write the game log path: {error}"));
            }
            self.write_boot_settings(&resolved, role == Role::Guest);
            self.write_settings_file();
        }
        let pids_before_launch = game_pids();
        match open_url(&format!("{STEAM_RUN_URL_PREFIX}{APP_ID}")) {
            Ok(()) => {
                self.log.push("asked Steam to start the game".to_owned());
                self.game_watch = Some(GameWatch::new(&pids_before_launch));
            }
            Err(error) => {
                self.log.push(format!("could not open Steam: {error}"));
                self.lobby_started = false;
                return false;
            }
        }
        true
    }

    fn write_boot_settings(&mut self, resolved: &ResolvedLaunch, write_body: bool) {
        let Some(game) = &self.game else {
            return;
        };
        let block_saves = false;
        let body = write_body.then_some(resolved.body);
        let coop_file = save_dir(game).and_then(|saves| coop_gamedata_path(&saves));
        self.log.push(match coop_file {
            Some(path) => format!(
                "co-op save file: {} ({})",
                path.display(),
                std::fs::metadata(&path).map_or_else(
                    |error| format!("missing: {error}"),
                    |meta| format!("{} bytes", meta.len())
                )
            ),
            None => "co-op save file: no save folder found".to_owned(),
        });
        let written = match &resolved.new_game {
            Some((slot, name)) => write_new_game(&game.game_dir, *slot, name, block_saves, body),
            None => write_boot(&game.game_dir, resolved.boot_slot, block_saves, body),
        };
        if let Err(error) = written {
            self.log
                .push(format!("could not write the boot settings: {error}"));
        }
    }

    fn plan_launch(&self, mode: u8) -> Option<ResolvedLaunch> {
        let new_game_name = self.name_check().ok();
        let plan = resolve_mode(mode, self.selected_slot, new_game_name)?;
        match plan {
            LaunchMode::Continue { slot } => Some(ResolvedLaunch {
                name: slot_player_name(&self.slots, Some(slot))
                    .unwrap_or_else(|| self.effective_player_name()),
                boot_slot: Some(slot),
                new_game: None,
                body: slot_body(&self.slot_bodies, slot),
            }),
            LaunchMode::NewGame { name } => Some(ResolvedLaunch {
                boot_slot: None,
                new_game: Some((new_game_slot(&self.slots)?, name.clone())),
                name,
                body: self.config.body,
            }),
        }
    }

    fn resolve_launch(&mut self, mode: u8) -> Option<ResolvedLaunch> {
        let new_game = self.plan_launch(mode)?.new_game.is_some();
        if new_game {
            if !self.game_running {
                self.ensure_coop_file();
            }
            self.refresh_saves();
        }
        let resolved = self.plan_launch(mode)?;
        if let Some((slot, name)) = &resolved.new_game {
            self.log.push(format!(
                "starting a new co-op game in slot {slot} as {name}"
            ));
            self.config.player_name = Some(name.clone());
            self.save_config();
            self.remember_slot_body(*slot, resolved.body);
        }
        Some(resolved)
    }

    fn remember_slot_body(&mut self, slot: u8, body: PartnerBody) {
        let Some(path) = self
            .game
            .as_ref()
            .and_then(save_dir)
            .and_then(|dir| coop_bodies_path(&dir))
        else {
            return;
        };
        match store_slot_body(&path, slot, body) {
            Ok(()) => {
                self.slot_bodies.insert(slot, body);
                self.log
                    .push(format!("co-op slot {slot} keeps its character: {body:?}"));
            }
            Err(error) => self
                .log
                .push(format!("could not store the slot's character: {error}")),
        }
    }

    fn ensure_coop_file(&mut self) {
        let Some(saves) = self.game.as_ref().and_then(save_dir) else {
            return;
        };
        let Some(target) = coop_gamedata_path(&saves) else {
            return;
        };
        if target.is_file() {
            return;
        }
        let vanilla = gamedata_path(&saves).and_then(|path| std::fs::read(path).ok());
        match fresh_coop_file(vanilla.as_deref()) {
            Ok(bytes) => match std::fs::write(&target, bytes) {
                Ok(()) => self.log.push("made a fresh co-op save file".to_owned()),
                Err(error) => self
                    .log
                    .push(format!("could not write the co-op save file: {error}")),
            },
            Err(SlotError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                self.log
                    .push("no normal save file yet: the game makes the co-op save".to_owned());
            }
            Err(error) => self
                .log
                .push(format!("could not make a fresh co-op save file: {error}")),
        }
    }

    fn fail_launch(&mut self, message: String) {
        self.log.push(message.clone());
        self.play_error = Some(message);
    }

    pub fn player_settings(&self) -> PlayerSettings {
        self.config.player_settings(crate::DEV_BUILD)
    }

    pub fn apply_settings(&mut self) {
        self.save_config();
        self.write_settings_file();
    }

    fn write_settings_file(&mut self) {
        let Some(game) = &self.game else {
            return;
        };
        if let Err(error) = write_settings(
            &game.game_dir,
            &self.player_settings(),
            &self.config.session_options(),
        ) {
            self.log
                .push(format!("could not write the game settings: {error}"));
        }
        if let Err(error) = write_dev_switches(&game.game_dir, crate::DEV_BUILD) {
            self.log
                .push(format!("could not write the dev switches: {error}"));
        }
    }

    fn launch_second_copy(&mut self, role: Role, resolved: &ResolvedLaunch) {
        if !crate::DEV_BUILD {
            self.log.push("the game is already running".to_owned());
            self.lobby_started = false;
            return;
        }
        self.play_error = None;
        let (Some(game), Some(port)) = (self.game.clone(), self.link_port) else {
            self.fail_second_copy("no game or no link port for a second copy".to_owned());
            return;
        };
        if let Some(message) = mod_mismatch(&game.game_dir, self.dll_source.as_deref()) {
            self.fail_second_copy(message);
            return;
        }
        self.write_boot_settings(resolved, false);
        self.write_settings_file();
        let log_path = self.start_game_log(role);
        let command = match build_command(
            &game,
            port,
            &resolved.name,
            resolved.boot_slot,
            resolved
                .new_game
                .as_ref()
                .map(|(slot, name)| (*slot, name.as_str())),
            (role == Role::Guest).then_some(resolved.body),
            log_path.as_deref(),
        ) {
            Ok(command) => command,
            Err(error) => {
                self.fail_second_copy(format!("could not start a second copy: {error}"));
                return;
            }
        };
        match start_second_copy(&command) {
            Ok(copy) => {
                self.log.push(format!(
                    "started a second game copy as the {}",
                    if role == Role::Host { "host" } else { "guest" }
                ));
                self.second_copy = Some(copy);
            }
            Err(error) => self.fail_second_copy(format!("could not start a second copy: {error}")),
        }
    }

    pub fn copy_launch_command(&mut self, ctx: &Context) {
        let (Some(game), Some(port)) = (self.game.clone(), self.link_port) else {
            self.log
                .push("no game or no link port to build the command".to_owned());
            return;
        };
        let Some(resolved) = self.plan_launch(self.lobby_mode) else {
            self.log
                .push("no valid name or save slot to build the command".to_owned());
            return;
        };
        match build_command(
            &game,
            port,
            &resolved.name,
            resolved.boot_slot,
            resolved
                .new_game
                .as_ref()
                .map(|(slot, name)| (*slot, name.as_str())),
            None,
            None,
        ) {
            Ok(command) => {
                ctx.copy_text(command.shell_line());
                self.log.push(format!(
                    "copied the launch command: {}",
                    command.shell_line()
                ));
            }
            Err(error) => self
                .log
                .push(format!("could not build the launch command: {error}")),
        }
    }

    pub fn export_logs(&mut self) {
        let Some(dir) = data_dir().map(|dir| dir.join(LOGS_FOLDER_NAME)) else {
            self.export_note = Some("no data folder on this system".to_owned());
            return;
        };
        let path = dir.join(format!("{}.txt", format_stamp(&chrono::Local::now())));
        let written = std::fs::create_dir_all(&dir)
            .and_then(|()| std::fs::write(&path, self.log.join("\n") + "\n"));
        self.export_note = Some(match written {
            Ok(()) => format!("saved {}", path.display()),
            Err(error) => format!("could not save the logs: {error}"),
        });
    }

    pub fn open_logs_folder(&mut self) {
        let Some(dir) = logs_dir() else {
            self.export_note = Some("no data folder on this system".to_owned());
            return;
        };
        let opened =
            std::fs::create_dir_all(&dir).and_then(|()| open_url(&dir.display().to_string()));
        if let Err(error) = opened {
            self.export_note = Some(format!("could not open the logs folder: {error}"));
        }
    }

    fn switch_launcher_log(&mut self, role: Role) {
        if self.role_log_chosen {
            return;
        }
        self.role_log_chosen = true;
        self.write_new_log_lines();
        let Some(dir) = logs_dir() else {
            return;
        };
        self.log_writer = LogWriter::start(&dir, launcher_log_name(role)).ok();
        self.log_written = 0;
        self.write_new_log_lines();
    }

    fn start_game_log(&mut self, role: Role) -> Option<String> {
        let path = logs_dir()?.join(game_log_name(role));
        if let Err(error) = std::fs::create_dir_all(path.parent()?).and_then(|()| rotate_log(&path))
        {
            self.log
                .push(format!("could not rotate the game log: {error}"));
        }
        Some(path_as_the_game_sees_it(&path))
    }

    fn write_new_log_lines(&mut self) {
        let Some(writer) = self.log_writer.as_mut() else {
            return;
        };
        for line in self.log.iter().skip(self.log_written) {
            let _ = writer.write_line(line);
        }
        self.log_written = self.log.len();
    }

    fn fail_second_copy(&mut self, message: String) {
        self.lobby_started = false;
        self.log.push(message.clone());
        self.play_error = Some(message);
    }

    fn poll_second_copy(&mut self) {
        let Some(copy) = &self.second_copy else {
            return;
        };
        let output = copy.take_output();
        self.log.extend(
            output
                .lines
                .into_iter()
                .map(|line| format!("[copy] {line}")),
        );
        let Some(exit) = output.exit else {
            return;
        };
        self.second_copy = None;
        self.mark_game_closed();
        if exit.seconds < EARLY_EXIT_SECONDS {
            self.play_error = Some(format!(
                "the second copy closed after {:.0} s (exit {}); see the log at the bottom or second-copy.log",
                exit.seconds,
                exit.code
                    .map_or_else(|| "none".to_owned(), |code| code.to_string())
            ));
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
        self.refresh_saves();
    }

    pub fn backup_coop_saves(&mut self) {
        let (Some(game), Some(root)) = (&self.game, coop_backup_root()) else {
            self.log
                .push("no game found, so no co-op saves to back up".to_owned());
            return;
        };
        let Some(saves) = save_dir(game) else {
            self.log
                .push("could not find the Documents folder".to_owned());
            return;
        };
        let stamp = format_stamp(&chrono::Local::now());
        match backup_coop_file(&saves, &root, &stamp) {
            Ok(_) => self.log.push(format!("saved co-op backup {stamp}")),
            Err(error) => self.log.push(format!("co-op backup failed: {error}")),
        }
        self.refresh_saves();
    }

    pub fn restore_selected_backup(&mut self) {
        let (Some(game), Some(root), Some(stamp)) =
            (&self.game, coop_backup_root(), self.selected_backup.clone())
        else {
            return;
        };
        if self.game_running {
            self.log
                .push("close the game before restoring saves".to_owned());
            return;
        }
        let Some(saves) = save_dir(game) else {
            self.log
                .push("could not find the Documents folder".to_owned());
            return;
        };
        let now = format_stamp(&chrono::Local::now());
        match restore_coop_backup(&saves, &root, &stamp, &now) {
            Ok(_) => self.log.push(format!(
                "restored co-op backup {stamp}; the co-op saves it replaced are in before-restore-{now}"
            )),
            Err(error) => self.log.push(format!("restore failed: {error}")),
        }
        self.refresh_saves();
    }

    pub fn copy_normal_slot_into_coop(&mut self) {
        let (Some(game), Some(from)) = (&self.game, self.copy_from_slot) else {
            return;
        };
        if self.game_running {
            self.log
                .push("close the game before copying a save".to_owned());
            return;
        }
        let Some(saves) = save_dir(game) else {
            self.log
                .push("could not find the Documents folder".to_owned());
            return;
        };
        let to = match coop_copy_target_slot(&saves) {
            Ok(slot) => slot,
            Err(error) => {
                self.log.push(format!("copy refused: {error}"));
                return;
            }
        };
        let stamp = format_stamp(&chrono::Local::now());
        if let Some(root) = coop_backup_root()
            && coop_gamedata_path(&saves).is_some_and(|path| path.is_file())
            && let Err(error) = backup_coop_file(&saves, &root, &format!("before-copy-{stamp}"))
        {
            self.log.push(format!(
                "copy refused: could not back up the co-op file first ({error})"
            ));
            return;
        }
        match copy_slot_into_coop(&saves, from, to) {
            Ok(_) => {
                self.log
                    .push(format!("copied normal slot {from} into co-op slot {to}"));
                self.remember_slot_body(to, PartnerBody::Nier);
            }
            Err(error) => self.log.push(format!("copy failed: {error}")),
        }
        self.refresh_saves();
    }

    pub fn apply_override(&mut self) {
        let text = self.override_text.trim();
        self.config.game_dir = (!text.is_empty()).then(|| PathBuf::from(text));
        self.save_config();
        self.refresh();
    }

    pub fn find_game(&mut self) {
        self.override_text.clear();
        self.apply_override();
        match &self.game {
            Some(game) => {
                self.override_text = game.game_dir.display().to_string();
                self.log.push("found the game".to_owned());
            }
            None => self
                .log
                .push("could not find the game: type the folder".to_owned()),
        }
    }

    pub(super) fn save_config(&mut self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Err(error) = self.config.save(&path) {
            self.log.push(format!("could not save settings: {error}"));
        }
    }
}

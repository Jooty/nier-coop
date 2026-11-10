//! Starts a two-copy dev session of NieR:Automata the way the launcher does, without its window.
//!
//! Local only. It installs the hook, writes the link settings, joins two launcher cores, starts both
//! game copies the way the launcher starts a second copy, watches them for a fixed time, then closes
//! the copies and uninstalls.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use nc_launcher::DEV_BUILD;
use nc_launcher::app::{Role, run_install, run_uninstall};
use nc_launcher::config::LauncherConfig;
use nc_launcher::game_link::{
    StartChoice, StartSetting, clear_boot, clear_link, write_character, write_dev_switches,
    write_link, write_log, write_settings, write_start,
};
use nc_launcher::install::{end_processes, game_pids};
use nc_launcher::logs::{LogWriter, game_log_name, path_as_the_game_sees_it};
use nc_launcher::proton::{USER_REG_BACKUP_SUFFIX, prefix_is_busy, remove_empty_override_keys};
use nc_launcher::second_copy::{
    SecondCopy, build_command, character_environment, start_environment, start_second_copy,
};
use nc_launcher::steam::{GameInstall, detect_game};
use nc_launcher_core::{
    AUTOMATA, LOBBY_MODE_NEW_GAME, LauncherCore, LauncherNet, LinkStatus, LobbyReady,
    TransportFactory,
};
use nc_net::{LoopbackNetwork, Transport};
use replicant_proto::PartnerBody;

const DRIVER_LOG_NAME: &str = "driver.log";
const HOST_NAME: &str = "Host";
const GUEST_NAME: &str = "Guest";
const HOST_CHARACTER_ID: &str = "2b";
const GUEST_CHARACTER_ID: &str = "9s";
const HOST_CHARACTER_INDEX: u8 = 0;
const GUEST_CHARACTER_INDEX: u8 = 1;
const MAX_RUN_SECONDS: u64 = 330;
const SECONDS_TO_WAIT_FOR_THE_HOST_PROCESS: u64 = 60;
const SECONDS_BETWEEN_HOST_PROCESS_AND_GUEST_START: u64 = 25;
const DEFAULT_SECONDS_BETWEEN_TIMED_SCREENSHOTS: u64 = 20;
const PARTNER_DRIVE_ENV: &str = "NIER_COOP_PARTNER_DRIVE";
const AUTOPILOT_ENV: &str = "NIER_COOP_AUTOPILOT";
const AUTOPILOT_CIRCLE_TEXT: &str = "circle";
const SECONDS_TO_WAIT_AFTER_TERM: u64 = 20;
const SECONDS_TO_WAIT_FOR_A_FREE_PREFIX: u64 = 90;
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const LOG_LINES_THAT_GET_A_SCREENSHOT: [&str; 16] = [
    "free: the local player",
    "focus: the window lost focus",
    "autopilot: first speed",
    "partner: the body is in state",
    "saves: co-op folder",
    "saves: NOT redirected",
    "start: the title is idle",
    "start: phase start accepted",
    "start: in the world",
    "start: refused",
    "net: connected to",
    "net: the other player plays",
    "partner: taking over",
    "partner: spawned a body",
    "input feed: first button write",
    "story: refused scene request",
];

struct Arguments {
    hook_dll: PathBuf,
    out_dir: PathBuf,
    run_seconds: u64,
    start: StartSetting,
    screenshot_program: Option<PathBuf>,
    seconds_between_screenshots: u64,
    partner_drive: Option<String>,
    autopilot: AutopilotCopies,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutopilotCopies {
    None,
    Host,
    Guest,
    Both,
}

struct Driver {
    log: LogWriter,
    out_dir: PathBuf,
    screenshot_program: Option<PathBuf>,
    user_reg_backup_was_there: bool,
}

struct Copy {
    role: Role,
    process: SecondCopy,
    output_log: LogWriter,
    game_log: PathBuf,
    game_log_bytes_read: usize,
    exited: bool,
}

struct Side {
    role: Role,
    core: LauncherCore,
    last_status: LinkStatus,
}

fn main() {
    let arguments = match parse_arguments() {
        Ok(arguments) => arguments,
        Err(message) => {
            eprintln!("{message}");
            eprintln!(
                "usage: first_run --hook <automata_hook.dll> --out <folder> [--seconds N] [--start world|title|slot] [--story hold|allow] [--shot <program>] [--shot-seconds N] [--partner-drive input|state] [--autopilot host|guest|both|none]"
            );
            std::process::exit(2);
        }
    };
    if let Err(error) = std::fs::create_dir_all(&arguments.out_dir) {
        eprintln!("cannot make {}: {error}", arguments.out_dir.display());
        std::process::exit(2);
    }
    let log = match LogWriter::start(&arguments.out_dir, DRIVER_LOG_NAME) {
        Ok(log) => log,
        Err(error) => {
            eprintln!("cannot open the driver log: {error}");
            std::process::exit(2);
        }
    };
    let mut driver = Driver {
        log,
        out_dir: arguments.out_dir.clone(),
        screenshot_program: arguments.screenshot_program.clone(),
        user_reg_backup_was_there: false,
    };
    let ok = driver.run(&arguments);
    driver.say(if ok {
        "driver: done"
    } else {
        "driver: stopped with a failure"
    });
    std::process::exit(if ok { 0 } else { 1 });
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut hook_dll = None;
    let mut out_dir = None;
    let mut run_seconds = MAX_RUN_SECONDS;
    let mut start = StartSetting::default();
    let mut screenshot_program = None;
    let mut seconds_between_screenshots = DEFAULT_SECONDS_BETWEEN_TIMED_SCREENSHOTS;
    let mut partner_drive = None;
    let mut autopilot = AutopilotCopies::None;
    let mut words = std::env::args().skip(1);
    while let Some(word) = words.next() {
        let mut value = || words.next().ok_or(format!("{word} needs a value"));
        match word.as_str() {
            "--hook" => hook_dll = Some(PathBuf::from(value()?)),
            "--out" => out_dir = Some(PathBuf::from(value()?)),
            "--shot" => screenshot_program = Some(PathBuf::from(value()?)),
            "--partner-drive" => partner_drive = Some(value()?),
            "--autopilot" => {
                autopilot = match value()?.as_str() {
                    "host" => AutopilotCopies::Host,
                    "guest" => AutopilotCopies::Guest,
                    "both" => AutopilotCopies::Both,
                    "none" => AutopilotCopies::None,
                    other => return Err(format!("unknown autopilot setting {other}")),
                };
            }
            "--shot-seconds" => {
                seconds_between_screenshots = value()?
                    .parse::<u64>()
                    .map_err(|error| error.to_string())?
                    .max(5);
            }
            "--seconds" => {
                run_seconds = value()?
                    .parse::<u64>()
                    .map_err(|error| error.to_string())?
                    .min(MAX_RUN_SECONDS);
            }
            "--start" => {
                start.choice = match value()?.as_str() {
                    "world" => StartChoice::World,
                    "title" => StartChoice::Title,
                    "slot" => StartChoice::Slot,
                    other => return Err(format!("unknown start {other}")),
                };
            }
            "--story" => {
                start.story_allowed = match value()?.as_str() {
                    "hold" => false,
                    "allow" => true,
                    other => return Err(format!("unknown story setting {other}")),
                };
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Arguments {
        hook_dll: hook_dll.ok_or("--hook is missing")?,
        out_dir: out_dir.ok_or("--out is missing")?,
        run_seconds,
        start,
        screenshot_program,
        seconds_between_screenshots,
        partner_drive,
        autopilot,
    })
}

impl AutopilotCopies {
    fn covers(self, role: Role) -> bool {
        match self {
            Self::None => false,
            Self::Both => true,
            Self::Host => role == Role::Host,
            Self::Guest => role == Role::Guest,
        }
    }
}

fn loopback_factory(network: &LoopbackNetwork) -> TransportFactory {
    let network = network.clone();
    Arc::new(move || Ok(Box::new(network.endpoint()) as Box<dyn Transport>))
}

fn role_text(role: Role) -> &'static str {
    role.text()
}

fn wait_until(seconds: u64, mut done: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if done() {
            return true;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
    done()
}

impl Driver {
    fn say(&mut self, line: &str) {
        println!("{line}");
        let _ = self.log.write_line(line);
    }

    fn run(&mut self, arguments: &Arguments) -> bool {
        self.say(&format!(
            "driver: dev build {DEV_BUILD}, hook {}, start {:?}, partner drive {:?}, autopilot {:?}, at most {} s",
            arguments.hook_dll.display(),
            arguments.start,
            arguments.partner_drive,
            arguments.autopilot,
            arguments.run_seconds
        ));
        let config = LauncherConfig::default();
        let Some(game) = detect_game(&AUTOMATA, &config) else {
            self.say("driver: the game was not found");
            return false;
        };
        self.say(&format!(
            "driver: game folder {}, exe {}, prefix {:?}",
            game.game_dir.display(),
            game.exe_path().display(),
            game.proton_prefix
        ));
        if !game_pids(&AUTOMATA).is_empty() {
            self.say("driver: the game is already running, nothing was touched");
            return false;
        }
        if !arguments.hook_dll.is_file() {
            self.say("driver: the hook DLL was not found, nothing was touched");
            return false;
        }
        self.user_reg_backup_was_there = user_reg_backup(&game).is_some_and(|path| path.exists());
        let report = run_install(game.clone(), Some(arguments.hook_dll.clone()));
        for line in &report.lines {
            self.say(&format!("install: {line}"));
        }
        if report.failed {
            self.uninstall(&game);
            return false;
        }
        let ran = self.run_session(&game, &config, arguments);
        self.close_game(&game);
        self.uninstall(&game);
        ran
    }

    fn run_session(
        &mut self,
        game: &GameInstall,
        config: &LauncherConfig,
        arguments: &Arguments,
    ) -> bool {
        let network = LoopbackNetwork::new();
        let mut sides = Vec::new();
        for (role, name) in [(Role::Host, HOST_NAME), (Role::Guest, GUEST_NAME)] {
            match LauncherCore::for_game(&AUTOMATA, name, loopback_factory(&network)) {
                Ok(core) => {
                    core.set_player_settings(config.player_settings(DEV_BUILD));
                    core.set_session_options(config.session_options());
                    self.say(&format!(
                        "{} core: link port {}",
                        role_text(role),
                        core.link_port()
                    ));
                    sides.push(Side {
                        role,
                        core,
                        last_status: LinkStatus::default(),
                    });
                }
                Err(error) => {
                    self.say(&format!(
                        "{} core: could not start: {error}",
                        role_text(role)
                    ));
                    return false;
                }
            }
        }
        let ticket = match sides[0].core.host() {
            Ok(ticket) => ticket,
            Err(error) => {
                self.say(&format!("host core: could not host: {error}"));
                return false;
            }
        };
        self.say("host core: hosting");
        if let Err(error) = sides[1].core.join(&ticket) {
            self.say(&format!("guest core: could not join: {error}"));
            return false;
        }
        sides[0].core.send_lobby_character(HOST_CHARACTER_INDEX);
        sides[1].core.send_lobby_character(GUEST_CHARACTER_INDEX);
        let ready = LobbyReady {
            ready: true,
            mode: LOBBY_MODE_NEW_GAME,
            slot: None,
        };
        sides[0].core.send_lobby_ready(ready);
        sides[1].core.send_lobby_ready(ready);
        wait_until(10, || {
            sides
                .iter()
                .all(|side| side.core.link_status().peer_name.is_some())
        });
        self.note_cores(&mut sides);

        self.write_link_file(game, config, sides[0].core.link_port(), arguments.start);
        let mut copies = Vec::new();
        let Some(host_copy) = self.start_copy(
            game,
            Role::Host,
            HOST_NAME,
            sides[0].core.link_port(),
            (HOST_CHARACTER_ID, GUEST_CHARACTER_ID),
            arguments,
        ) else {
            return false;
        };
        copies.push(host_copy);
        sides[0].core.start_lobby(LOBBY_MODE_NEW_GAME);
        let host_seen = wait_until(SECONDS_TO_WAIT_FOR_THE_HOST_PROCESS, || {
            !game_pids(&AUTOMATA).is_empty()
        });
        self.say(&format!(
            "driver: host game process seen: {host_seen}, pids {:?}",
            game_pids(&AUTOMATA)
        ));
        let started = Instant::now();
        let run_for = Duration::from_secs(arguments.run_seconds);
        let guest_due = Duration::from_secs(SECONDS_BETWEEN_HOST_PROCESS_AND_GUEST_START);
        let mut last_timed_screenshot = Instant::now();
        let mut guest_tried = false;
        let mut screenshot_number = 0_u32;
        while started.elapsed() < run_for {
            std::thread::sleep(POLL_INTERVAL);
            if !guest_tried && host_seen && started.elapsed() >= guest_due {
                guest_tried = true;
                if let Some(copy) = self.start_copy(
                    game,
                    Role::Guest,
                    GUEST_NAME,
                    sides[1].core.link_port(),
                    (GUEST_CHARACTER_ID, HOST_CHARACTER_ID),
                    arguments,
                ) {
                    copies.push(copy);
                }
            }
            self.note_cores(&mut sides);
            let mut reasons = Vec::new();
            for copy in &mut copies {
                reasons.extend(self.note_copy(copy));
            }
            if last_timed_screenshot.elapsed().as_secs() >= arguments.seconds_between_screenshots {
                reasons.push("timed".to_owned());
            }
            if !reasons.is_empty() {
                last_timed_screenshot = Instant::now();
                screenshot_number += 1;
                self.take_screenshots(screenshot_number, started.elapsed(), &reasons);
            }
            let all_exited = copies.iter().all(|copy| copy.exited);
            if all_exited && (guest_tried || !host_seen) {
                self.say("driver: every copy has exited");
                break;
            }
        }
        self.say(&format!(
            "driver: watch ended after {:.0} s, game pids {:?}",
            started.elapsed().as_secs_f64(),
            game_pids(&AUTOMATA)
        ));
        for copy in &copies {
            copy.process.kill_tree();
        }
        wait_until(SECONDS_TO_WAIT_AFTER_TERM, || {
            game_pids(&AUTOMATA).is_empty()
        });
        for copy in &mut copies {
            self.note_copy(copy);
        }
        self.note_cores(&mut sides);
        true
    }

    fn write_link_file(
        &mut self,
        game: &GameInstall,
        config: &LauncherConfig,
        port: u16,
        start: StartSetting,
    ) {
        let log_path = path_as_the_game_sees_it(&self.out_dir.join(game_log_name(Role::Host)));
        let results = [
            write_link(game.profile, &game.game_dir, port, HOST_NAME),
            write_log(game.profile, &game.game_dir, &log_path),
            write_character(
                game.profile,
                &game.game_dir,
                role_text(Role::Host),
                HOST_CHARACTER_ID,
                Some(GUEST_CHARACTER_ID),
            ),
            write_start(game.profile, &game.game_dir, start),
            write_settings(
                game.profile,
                &game.game_dir,
                &config.player_settings(DEV_BUILD),
                &config.session_options(),
            ),
            write_dev_switches(game.profile, &game.game_dir, DEV_BUILD),
        ];
        for error in results.into_iter().filter_map(Result::err) {
            self.say(&format!("link file: could not write: {error}"));
        }
        let text = std::fs::read_to_string(game.game_dir.join(game.profile.link_file_name))
            .unwrap_or_default();
        self.say(&format!("link file: {}", text.replace('\n', "; ")));
    }

    fn start_copy(
        &mut self,
        game: &GameInstall,
        role: Role,
        name: &str,
        port: u16,
        (own_character, partner_character): (&str, &str),
        arguments: &Arguments,
    ) -> Option<Copy> {
        let game_log = self.out_dir.join(game_log_name(role));
        let body = (role == Role::Guest).then_some(PartnerBody::Nier);
        let mut command = match build_command(
            game,
            port,
            name,
            None,
            None,
            body,
            Some(&path_as_the_game_sees_it(&game_log)),
        ) {
            Ok(command) => command,
            Err(error) => {
                self.say(&format!(
                    "{} copy: could not build the command: {error}",
                    role_text(role)
                ));
                return None;
            }
        };
        command.envs.extend(character_environment(
            role_text(role),
            own_character,
            Some(partner_character),
        ));
        command.envs.extend(start_environment(arguments.start));
        if let Some(partner_drive) = &arguments.partner_drive {
            command
                .envs
                .push((PARTNER_DRIVE_ENV.to_owned(), partner_drive.clone()));
        }
        if arguments.autopilot.covers(role) {
            command
                .envs
                .push((AUTOPILOT_ENV.to_owned(), AUTOPILOT_CIRCLE_TEXT.to_owned()));
        }
        let output_log_name = format!("copy-{}-output.log", role_text(role));
        let output_log = match LogWriter::start(&self.out_dir, &output_log_name) {
            Ok(log) => log,
            Err(error) => {
                self.say(&format!("cannot open {output_log_name}: {error}"));
                return None;
            }
        };
        match start_second_copy(&command) {
            Ok(process) => {
                self.say(&format!(
                    "{} copy: started: {}",
                    role_text(role),
                    process.command_line
                ));
                Some(Copy {
                    role,
                    process,
                    output_log,
                    game_log,
                    game_log_bytes_read: 0,
                    exited: false,
                })
            }
            Err(error) => {
                self.say(&format!(
                    "{} copy: could not start: {error}",
                    role_text(role)
                ));
                None
            }
        }
    }

    fn note_cores(&mut self, sides: &mut [Side]) {
        for side in sides {
            for event in side.core.drain_events() {
                self.say(&format!("{} core: event {event:?}", role_text(side.role)));
            }
            let mut status = side.core.link_status();
            status.rtt = None;
            status.ticket = None;
            if status != side.last_status {
                self.say(&format!("{} core: status {status:?}", role_text(side.role)));
                side.last_status = status;
            }
        }
    }

    fn note_copy(&mut self, copy: &mut Copy) -> Vec<String> {
        let output = copy.process.take_output();
        for line in &output.lines {
            let _ = copy.output_log.write_line(line);
        }
        if let Some(exit) = output.exit {
            copy.exited = true;
            self.say(&format!(
                "{} copy: exited with {:?} after {:.1} s",
                role_text(copy.role),
                exit.code,
                exit.seconds
            ));
        }
        let bytes = std::fs::read(&copy.game_log).unwrap_or_default();
        let Some(new_bytes) = bytes.get(copy.game_log_bytes_read..) else {
            return Vec::new();
        };
        let Some(complete) = new_bytes.iter().rposition(|byte| *byte == b'\n') else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&new_bytes[..=complete]).into_owned();
        copy.game_log_bytes_read += complete + 1;
        let mut reasons = Vec::new();
        for line in text.lines() {
            if let Some(marker) = LOG_LINES_THAT_GET_A_SCREENSHOT
                .iter()
                .find(|marker| line.contains(**marker))
            {
                self.say(&format!("{} game log: {line}", role_text(copy.role)));
                reasons.push(format!("{}: {marker}", role_text(copy.role)));
            }
        }
        reasons
    }

    fn take_screenshots(&mut self, number: u32, elapsed: Duration, reasons: &[String]) {
        let Some(program) = self.screenshot_program.clone() else {
            return;
        };
        let label = format!("{number:03}-{:03}s", elapsed.as_secs());
        let result = Command::new(&program)
            .arg(&self.out_dir)
            .arg(&label)
            .output();
        let outcome = match result {
            Ok(output) => String::from_utf8_lossy(&output.stdout)
                .trim()
                .replace('\n', "; "),
            Err(error) => format!("could not run the screenshot program: {error}"),
        };
        self.say(&format!(
            "screenshot {label} ({}): {outcome}",
            reasons.join(", ")
        ));
    }

    fn close_game(&mut self, game: &GameInstall) {
        let left = game_pids(&AUTOMATA);
        if !left.is_empty() {
            self.say(&format!(
                "driver: game processes still alive after the wait, ending them: {left:?}"
            ));
            end_processes(&left);
        }
        let Some(prefix) = &game.proton_prefix else {
            return;
        };
        let free = wait_until(SECONDS_TO_WAIT_FOR_A_FREE_PREFIX, || {
            !prefix_is_busy(prefix)
        });
        self.say(&format!("driver: the prefix is free: {free}"));
    }

    fn uninstall(&mut self, game: &GameInstall) {
        let link_file = game.game_dir.join(game.profile.link_file_name);
        if let Err(error) = clear_link(game.profile, &game.game_dir)
            .and_then(|()| clear_boot(game.profile, &game.game_dir))
        {
            self.say(&format!("uninstall: could not clean up the link: {error}"));
        }
        if link_file.is_file() {
            let left = std::fs::read_to_string(&link_file).unwrap_or_default();
            self.say(&format!(
                "uninstall: the link file still held: {}; removing it",
                left.replace('\n', "; ")
            ));
            if let Err(error) = std::fs::remove_file(&link_file) {
                self.say(&format!(
                    "uninstall: could not remove the link file: {error}"
                ));
            }
        }
        let report = run_uninstall(game.clone());
        for line in &report.lines {
            self.say(&format!("uninstall: {line}"));
        }
        match remove_empty_override_keys(game) {
            Ok(deleted) => self.say(&format!(
                "uninstall: registry keys deleted because they were left empty: {}",
                if deleted.is_empty() {
                    "none".to_owned()
                } else {
                    deleted.join(", ")
                }
            )),
            Err(error) => self.say(&format!(
                "uninstall: could not delete the empty registry keys: {error}"
            )),
        }
        self.remove_the_user_reg_backup_this_run_made(game);
        self.say(&format!(
            "uninstall: game folder now holds: {}",
            folder_listing(&game.game_dir)
        ));
    }

    fn remove_the_user_reg_backup_this_run_made(&mut self, game: &GameInstall) {
        let Some(backup) = user_reg_backup(game).filter(|path| path.exists()) else {
            return;
        };
        if self.user_reg_backup_was_there {
            self.say("uninstall: a registry backup was there before this run and is kept");
            return;
        }
        let removed = std::fs::remove_file(&backup);
        self.say(&format!(
            "uninstall: removed the registry backup this run made ({}): {}",
            backup.display(),
            removed.map_or_else(|error| error.to_string(), |()| "done".to_owned())
        ));
    }
}

fn user_reg_backup(game: &GameInstall) -> Option<PathBuf> {
    game.proton_prefix
        .as_ref()
        .map(|prefix| prefix.join(format!("user.reg.{USER_REG_BACKUP_SUFFIX}")))
}

fn folder_listing(dir: &Path) -> String {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names.join(", ")
}

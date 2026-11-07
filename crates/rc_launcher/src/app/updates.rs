use std::path::{Path, PathBuf};

use eframe::egui::{Context, ViewportCommand};

use crate::DEV_BUILD;
use crate::app::{Launcher, Task, run_install};
use crate::install::{HOOK_DLL_NAME, ModStatus};
use crate::steam::GameInstall;
use crate::update::{
    AvailableUpdate, UPDATE_PUBLIC_KEY_HEX, UPDATE_URL_FROM_BUILD, UpdateError, apply_update,
    check_for_update, own_build, short_commit,
};

type CheckResult = Result<Option<AvailableUpdate>, UpdateError>;
type ApplyResult = Result<Vec<String>, UpdateError>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Box<AvailableUpdate>),
    Updating,
    Failed(String),
}

#[derive(Default)]
pub struct Updater {
    pub state: UpdateState,
    check_task: Option<Task<CheckResult>>,
    apply_task: Option<Task<ApplyResult>>,
    started_at_launch: bool,
    exe_before_update: Option<PathBuf>,
}

impl Launcher {
    pub fn update_url(&self) -> Option<String> {
        if DEV_BUILD {
            return None;
        }
        self.config
            .update_url
            .as_deref()
            .or(UPDATE_URL_FROM_BUILD)
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_owned)
    }

    pub fn start_update_check(&mut self, ctx: &Context) {
        let Some(url) = self.update_url() else {
            return;
        };
        if matches!(
            self.updater.state,
            UpdateState::Checking | UpdateState::Updating
        ) {
            return;
        }
        self.updater.state = UpdateState::Checking;
        self.updater.check_task = Some(Task::spawn(
            move || {
                check_for_update(
                    &url,
                    UPDATE_PUBLIC_KEY_HEX,
                    &own_build(),
                    std::env::consts::OS,
                )
            },
            ctx.clone(),
        ));
    }

    pub fn start_update_apply(&mut self, ctx: &Context) {
        let UpdateState::Available(update) = self.updater.state.clone() else {
            return;
        };
        let Ok(exe) = std::env::current_exe() else {
            self.updater.state = UpdateState::Failed("could not find the launcher file".to_owned());
            return;
        };
        let reinstall_into = (!self.game_running
            && matches!(self.mod_status, ModStatus::Current | ModStatus::Outdated))
        .then(|| self.game.clone())
        .flatten();
        self.log.push(format!(
            "updating to {}...",
            short_commit(&update.manifest.commit)
        ));
        self.updater.state = UpdateState::Updating;
        self.updater.exe_before_update = Some(exe.clone());
        self.updater.apply_task = Some(Task::spawn(
            move || run_update(&update, &exe, reinstall_into),
            ctx.clone(),
        ));
    }

    pub(super) fn tick_updates(&mut self, ctx: &Context) {
        if !self.updater.started_at_launch {
            self.updater.started_at_launch = true;
            if self.config.check_for_updates && self.update_url().is_some() {
                self.start_update_check(ctx);
            }
        }
        if let Some(result) = self.updater.check_task.as_ref().and_then(Task::try_take) {
            self.updater.check_task = None;
            self.finish_check(result);
        }
        if let Some(result) = self.updater.apply_task.as_ref().and_then(Task::try_take) {
            self.updater.apply_task = None;
            self.finish_apply(result, ctx);
        }
    }

    fn finish_check(&mut self, result: CheckResult) {
        self.updater.state = match result {
            Ok(Some(update)) => {
                self.log.push(format!(
                    "update available: {}",
                    short_commit(&update.manifest.commit)
                ));
                UpdateState::Available(Box::new(update))
            }
            Ok(None) => UpdateState::UpToDate,
            Err(error) => {
                self.log.push(format!("update check failed: {error}"));
                UpdateState::Failed(error.to_string())
            }
        };
    }

    fn finish_apply(&mut self, result: ApplyResult, ctx: &Context) {
        match result {
            Ok(lines) => {
                self.log.extend(lines);
                self.log.push("updated, restarting the launcher".to_owned());
                self.updater.state = UpdateState::Idle;
                restart_launcher(self.updater.exe_before_update.take(), ctx);
            }
            Err(error) => {
                self.log.push(format!("update failed: {error}"));
                self.updater.state = UpdateState::Failed(error.to_string());
            }
        }
    }
}

fn run_update(
    update: &AvailableUpdate,
    exe: &Path,
    reinstall_into: Option<GameInstall>,
) -> ApplyResult {
    apply_update(update, exe)?;
    let mut lines = vec!["downloaded and verified the new launcher and hook dll".to_owned()];
    if let Some(game) = reinstall_into
        && let Some(dll) = exe.parent().map(|dir| dir.join(HOOK_DLL_NAME))
    {
        let report = run_install(game, Some(dll));
        lines.extend(report.lines);
    }
    Ok(lines)
}

fn restart_launcher(exe: Option<PathBuf>, ctx: &Context) {
    if let Some(exe) = exe {
        let _ = std::process::Command::new(exe)
            .args(std::env::args_os().skip(1))
            .spawn();
    }
    ctx.send_viewport_cmd(ViewportCommand::Close);
}

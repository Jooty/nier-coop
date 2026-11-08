use std::time::SystemTime;

use eframe::egui::Context;

use rc_launcher_core::{
    BundleHeader, log_upload_offered, new_install_id, pick_since, pick_unsent, role_of_files,
    upload_file_name, upload_url,
};

use crate::app::{Launcher, Role, Task};
use crate::logs::{
    UploadJob, UploadOutcome, list_log_entries, logs_dir, read_sent_marker, run_upload,
    unix_seconds,
};
use crate::update::{BUILD_COMMIT, build_config};

const STAMP_FORMAT: &str = "%Y%m%dT%H%M%SZ";
const STARTED_FORMAT: &str = "%Y-%m-%d %H:%M:%S UTC";

pub struct LogUploader {
    launcher_started_at: SystemTime,
    run_started_at: Option<SystemTime>,
    run_role: Option<Role>,
    task: Option<Task<UploadOutcome>>,
    unsent_checked: bool,
}

impl LogUploader {
    pub fn new(launcher_started_at: SystemTime) -> Self {
        Self {
            launcher_started_at,
            run_started_at: None,
            run_role: None,
            task: None,
            unsent_checked: false,
        }
    }
}

impl Launcher {
    pub fn log_upload_available(&self) -> bool {
        log_upload_offered(
            self.update_url().as_deref(),
            build_config().log_token.as_deref(),
        )
    }

    pub fn log_upload_busy(&self) -> bool {
        self.log_uploader.task.is_some()
    }

    pub(super) fn note_run_started(&mut self, role: Role) {
        self.log_uploader.run_started_at = Some(SystemTime::now());
        self.log_uploader.run_role = Some(role);
    }

    pub(super) fn upload_after_game(&mut self, ctx: &Context) {
        let Some(started) = self.log_uploader.run_started_at.take() else {
            return;
        };
        if !self.log_upload_available() || !self.config.send_logs {
            return;
        }
        let Some(dir) = logs_dir() else {
            return;
        };
        let names = pick_since(&list_log_entries(&dir), unix_seconds(started));
        let role = role_text(self.log_uploader.run_role, &names);
        self.start_upload(ctx, names, role);
    }

    pub fn send_logs_now(&mut self, ctx: &Context) {
        if !self.log_upload_available() || self.log_upload_busy() {
            return;
        }
        let Some(dir) = logs_dir() else {
            return;
        };
        let since = self
            .log_uploader
            .run_started_at
            .unwrap_or(self.log_uploader.launcher_started_at);
        let names = pick_since(&list_log_entries(&dir), unix_seconds(since));
        if names.is_empty() {
            self.log.push("no logs to send yet".to_owned());
            return;
        }
        let role = role_text(self.log_uploader.run_role, &names);
        self.start_upload(ctx, names, role);
    }

    pub(super) fn tick_log_upload(&mut self, ctx: &Context) {
        if let Some(outcome) = self.log_uploader.task.as_ref().and_then(Task::try_take) {
            self.log_uploader.task = None;
            self.log.push(match outcome {
                UploadOutcome::Sent { kilobytes } => {
                    format!("logs sent to the build server ({kilobytes} KB)")
                }
                UploadOutcome::Failed(reason) => format!("could not send logs: {reason}"),
            });
        }
        if std::mem::replace(&mut self.log_uploader.unsent_checked, true) {
            return;
        }
        if !self.log_upload_available() || !self.config.send_logs {
            return;
        }
        let Some(dir) = logs_dir() else {
            return;
        };
        let started = unix_seconds(self.log_uploader.launcher_started_at);
        let names = pick_unsent(&list_log_entries(&dir), read_sent_marker(&dir), started);
        let role = role_of_files(&names);
        self.start_upload(ctx, names, role);
    }

    fn start_upload(&mut self, ctx: &Context, names: Vec<String>, role: &str) {
        if names.is_empty() || self.log_upload_busy() {
            return;
        }
        let (Some(dir), Some(update_url), Some(token)) = (
            logs_dir(),
            self.update_url(),
            build_config().log_token.as_deref().map(str::trim),
        ) else {
            return;
        };
        let install_id = self.install_id();
        let now = chrono::Utc::now();
        let Some(url) = upload_file_name(&now.format(STAMP_FORMAT).to_string(), role)
            .and_then(|name| upload_url(&update_url, &install_id, &name))
        else {
            self.log
                .push("could not send logs: no valid upload address".to_owned());
            return;
        };
        let started: chrono::DateTime<chrono::Utc> = self.log_uploader.launcher_started_at.into();
        let job = UploadJob {
            dir,
            names,
            header: BundleHeader {
                install_id,
                player_name: self.effective_player_name(),
                role: role.to_owned(),
                build_commit: BUILD_COMMIT.to_owned(),
                os: std::env::consts::OS.to_owned(),
                launcher_started: started.format(STARTED_FORMAT).to_string(),
            },
            url,
            token: token.to_owned(),
            sent_marker_secs: now.timestamp().max(0).unsigned_abs(),
        };
        self.log_uploader.task = Some(Task::spawn(move || run_upload(&job), ctx.clone()));
    }

    fn install_id(&mut self) -> String {
        if let Some(id) = &self.config.install_id {
            return id.clone();
        }
        let id = new_install_id();
        self.config.install_id = Some(id.clone());
        self.save_config();
        id
    }
}

fn role_text(role: Option<Role>, names: &[String]) -> &'static str {
    match role {
        Some(Role::Host) => "host",
        Some(Role::Guest) => "guest",
        None => role_of_files(names),
    }
}

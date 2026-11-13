use std::sync::Arc;
use std::time::Instant;

use eframe::egui::Context;
use nc_launcher_core::{ConsoleClock, ConsoleTask};

use crate::app::{ConsoleRun, JoinDialog, LobbyFailure, Task};
use crate::net::{LauncherNet, NetError, SessionOptions};

pub struct NetForms {
    pub net: Arc<dyn LauncherNet>,
    pub join_ticket: String,
    pub join_dialog: Option<JoinDialog>,
    pub console: Option<ConsoleRun>,
    pub lobby_was_open: bool,
    pub lobby_failure: Option<LobbyFailure>,
    pub session_options: SessionOptions,
    pub host_result: Option<Result<String, NetError>>,
    pub join_result: Option<Result<(), NetError>>,
    host_task: Option<Task<Result<String, NetError>>>,
    join_task: Option<Task<Result<(), NetError>>>,
    abandoned: bool,
}

impl NetForms {
    pub fn new(net: Arc<dyn LauncherNet>) -> Self {
        Self {
            net,
            join_ticket: String::new(),
            join_dialog: None,
            console: None,
            lobby_was_open: false,
            lobby_failure: None,
            session_options: SessionOptions::default(),
            host_result: None,
            join_result: None,
            host_task: None,
            join_task: None,
            abandoned: false,
        }
    }

    pub fn poll(&mut self) {
        if let Some(result) = self.host_task.as_ref().and_then(Task::try_take) {
            self.host_task = None;
            self.host_result = Some(result);
        }
        if let Some(result) = self.join_task.as_ref().and_then(Task::try_take) {
            self.join_task = None;
            self.join_result = Some(result);
        }
        // A host or join given up on may still open a session when its call returns; close it then.
        if self.abandoned && !self.hosting_pending() && !self.joining_pending() {
            self.abandoned = false;
            self.leave();
        }
    }

    pub fn hosting_pending(&self) -> bool {
        self.host_task.is_some()
    }

    pub fn joining_pending(&self) -> bool {
        self.join_task.is_some()
    }

    pub fn start_host(&mut self, ctx: &Context, now_ms: f64) {
        let started_at = Instant::now();
        let net = Arc::clone(&self.net);
        self.host_result = None;
        self.lobby_failure = None;
        self.net.set_session_options(self.session_options);
        self.host_task = Some(Task::spawn(move || net.host(), ctx.clone()));
        self.console = Some(ConsoleRun {
            task: ConsoleTask::Hosting,
            clock: ConsoleClock::new(now_ms),
            started_at,
        });
    }

    pub fn push_session_options(&self) {
        self.net.set_session_options(self.session_options);
    }

    pub fn start_join(&mut self, ctx: &Context, now_ms: f64) {
        let started_at = Instant::now();
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        self.join_result = None;
        self.lobby_failure = None;
        self.join_dialog = None;
        self.join_task = Some(Task::spawn(move || net.join(&ticket), ctx.clone()));
        self.console = Some(ConsoleRun {
            task: ConsoleTask::Joining,
            clock: ConsoleClock::new(now_ms),
            started_at,
        });
    }

    /// Drops a host or a join still under way, or one that failed, and the console that covered it.
    pub fn abandon(&mut self) {
        self.console = None;
        self.abandoned = self.hosting_pending() || self.joining_pending();
        self.leave();
    }

    pub fn leave(&mut self) {
        self.net.leave();
        self.host_result = None;
        self.join_result = None;
    }
}

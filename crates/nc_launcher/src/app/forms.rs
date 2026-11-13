use std::sync::Arc;

use eframe::egui::Context;
use nc_launcher_core::{ConsoleClock, ConsoleTask};

use crate::app::{ConsoleRun, JoinDialog, Task};
use crate::net::{ConnectionReport, JOIN_PRECHECK_SECONDS, LauncherNet, NetError, SessionOptions};

pub struct NetForms {
    pub net: Arc<dyn LauncherNet>,
    pub join_ticket: String,
    pub join_dialog: Option<JoinDialog>,
    pub console: Option<ConsoleRun>,
    pub lobby_was_open: bool,
    pub session_options: SessionOptions,
    pub host_result: Option<Result<String, NetError>>,
    pub probe_result: Option<Result<ConnectionReport, NetError>>,
    pub join_result: Option<Result<(), NetError>>,
    host_task: Option<Task<Result<String, NetError>>>,
    join_task: Option<Task<Result<(), NetError>>>,
    precheck_task: Option<Task<Result<ConnectionReport, NetError>>>,
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
            session_options: SessionOptions::default(),
            host_result: None,
            probe_result: None,
            join_result: None,
            host_task: None,
            join_task: None,
            precheck_task: None,
            abandoned: false,
        }
    }

    pub fn poll(&mut self, ctx: &Context) -> bool {
        if let Some(result) = self.host_task.as_ref().and_then(Task::try_take) {
            self.host_task = None;
            self.host_result = Some(result);
        }
        if let Some(result) = self.join_task.as_ref().and_then(Task::try_take) {
            self.join_task = None;
            self.join_result = Some(result);
        }
        if self.abandoned {
            // A host or join given up on may still open a session when its call returns; close it then.
            if self
                .precheck_task
                .as_ref()
                .and_then(Task::try_take)
                .is_some()
            {
                self.precheck_task = None;
            }
            if !self.hosting_pending() && !self.joining_pending() {
                self.abandoned = false;
                self.leave();
            }
            return false;
        }
        let Some(result) = self.precheck_task.as_ref().and_then(Task::try_take) else {
            return false;
        };
        self.precheck_task = None;
        let passed = matches!(&result, Ok(report) if precheck_failure(report).is_none());
        self.probe_result = Some(result);
        if passed {
            self.spawn_join(ctx);
        }
        passed
    }

    pub fn hosting_pending(&self) -> bool {
        self.host_task.is_some()
    }

    pub fn joining_pending(&self) -> bool {
        self.join_task.is_some() || self.precheck_task.is_some()
    }

    pub fn start_host(&mut self, ctx: &Context, now_ms: f64) {
        let net = Arc::clone(&self.net);
        self.host_result = None;
        self.net.set_session_options(self.session_options);
        self.host_task = Some(Task::spawn(move || net.host(), ctx.clone()));
        self.console = Some(ConsoleRun {
            task: ConsoleTask::Hosting,
            clock: ConsoleClock::new(now_ms),
        });
    }

    pub fn push_session_options(&self) {
        self.net.set_session_options(self.session_options);
    }

    pub fn start_join(&mut self, ctx: &Context, now_ms: f64) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        self.probe_result = None;
        self.join_result = None;
        self.join_dialog = None;
        self.precheck_task = Some(Task::spawn(
            move || net.test_connection(&ticket, JOIN_PRECHECK_SECONDS),
            ctx.clone(),
        ));
        self.console = Some(ConsoleRun {
            task: ConsoleTask::Joining,
            clock: ConsoleClock::new(now_ms),
        });
    }

    fn spawn_join(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        self.join_task = Some(Task::spawn(move || net.join(&ticket), ctx.clone()));
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
        self.probe_result = None;
        self.join_result = None;
    }
}

pub fn precheck_failure(report: &ConnectionReport) -> Option<String> {
    if report.reachable && report.rtt.is_some() {
        None
    } else {
        Some(report.verdict.clone())
    }
}

use std::sync::Arc;

use eframe::egui::Context;

use crate::app::{EntryChoice, Task};
use crate::net::{ConnectionReport, JOIN_PRECHECK_SECONDS, LauncherNet, NetError, SessionOptions};

pub struct NetForms {
    pub net: Arc<dyn LauncherNet>,
    pub entry_choice: EntryChoice,
    pub join_ticket: String,
    pub session_options: SessionOptions,
    pub host_result: Option<Result<String, NetError>>,
    pub join_result: Option<Result<(), NetError>>,
    host_task: Option<Task<Result<String, NetError>>>,
    join_task: Option<Task<Result<(), NetError>>>,
    precheck_task: Option<Task<Result<ConnectionReport, NetError>>>,
}

impl NetForms {
    pub fn new(net: Arc<dyn LauncherNet>) -> Self {
        Self {
            net,
            entry_choice: EntryChoice::Host,
            join_ticket: String::new(),
            session_options: SessionOptions::default(),
            host_result: None,
            join_result: None,
            host_task: None,
            join_task: None,
            precheck_task: None,
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
        let Some(result) = self.precheck_task.as_ref().and_then(Task::try_take) else {
            return false;
        };
        self.precheck_task = None;
        let failure = match result {
            Ok(report) => precheck_failure(&report),
            Err(error) => Some(error.to_string()),
        };
        match failure {
            Some(text) => {
                self.join_result = Some(Err(NetError::Failed(text)));
                false
            }
            None => {
                self.spawn_join(ctx);
                true
            }
        }
    }

    pub fn hosting_pending(&self) -> bool {
        self.host_task.is_some()
    }

    pub fn joining_pending(&self) -> bool {
        self.join_task.is_some() || self.precheck_task.is_some()
    }

    pub fn start_host(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        self.host_result = None;
        self.net.set_session_options(self.session_options);
        self.host_task = Some(Task::spawn(move || net.host(), ctx.clone()));
    }

    pub fn push_session_options(&self) {
        self.net.set_session_options(self.session_options);
    }

    pub fn start_join(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        self.join_result = None;
        self.precheck_task = Some(Task::spawn(
            move || net.test_connection(&ticket, JOIN_PRECHECK_SECONDS),
            ctx.clone(),
        ));
    }

    fn spawn_join(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        self.join_task = Some(Task::spawn(move || net.join(&ticket), ctx.clone()));
    }

    pub fn leave(&mut self) {
        self.net.leave();
        self.host_result = None;
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

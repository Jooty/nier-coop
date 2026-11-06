use std::sync::Arc;

use eframe::egui::Context;

use crate::app::Task;
use crate::net::{ConnectionReport, DEFAULT_TEST_SECONDS, LauncherNet, NetError};

pub struct NetForms {
    pub net: Arc<dyn LauncherNet>,
    pub host_password: String,
    pub join_ticket: String,
    pub join_password: String,
    pub test_seconds: u32,
    pub host_result: Option<Result<String, NetError>>,
    pub join_result: Option<Result<(), NetError>>,
    pub test_result: Option<Result<ConnectionReport, NetError>>,
    host_task: Option<Task<Result<String, NetError>>>,
    join_task: Option<Task<Result<(), NetError>>>,
    test_task: Option<Task<Result<ConnectionReport, NetError>>>,
}

impl NetForms {
    pub fn new(net: Arc<dyn LauncherNet>) -> Self {
        Self {
            net,
            host_password: String::new(),
            join_ticket: String::new(),
            join_password: String::new(),
            test_seconds: DEFAULT_TEST_SECONDS,
            host_result: None,
            join_result: None,
            test_result: None,
            host_task: None,
            join_task: None,
            test_task: None,
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
        if let Some(result) = self.test_task.as_ref().and_then(Task::try_take) {
            self.test_task = None;
            self.test_result = Some(result);
        }
    }

    pub fn hosting_pending(&self) -> bool {
        self.host_task.is_some()
    }

    pub fn joining_pending(&self) -> bool {
        self.join_task.is_some()
    }

    pub fn testing_pending(&self) -> bool {
        self.test_task.is_some()
    }

    pub fn start_host(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let password = optional_password(&self.host_password);
        self.host_result = None;
        self.host_task = Some(Task::spawn(
            move || net.host(password.as_deref()),
            ctx.clone(),
        ));
    }

    pub fn start_join(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        let password = optional_password(&self.join_password);
        self.join_result = None;
        self.join_task = Some(Task::spawn(
            move || net.join(&ticket, password.as_deref()),
            ctx.clone(),
        ));
    }

    pub fn start_test(&mut self, ctx: &Context) {
        let net = Arc::clone(&self.net);
        let ticket = self.join_ticket.trim().to_owned();
        let password = optional_password(&self.join_password);
        let seconds = self.test_seconds;
        self.test_result = None;
        self.test_task = Some(Task::spawn(
            move || net.test_connection(&ticket, password.as_deref(), seconds),
            ctx.clone(),
        ));
    }

    pub fn leave(&mut self) {
        self.net.leave();
        self.host_result = None;
        self.join_result = None;
    }
}

fn optional_password(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_owned())
}

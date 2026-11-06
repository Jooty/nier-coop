use eframe::egui::{self, Color32, RichText, Ui};

use crate::app::{Launcher, NetForms};
use crate::install::ModStatus;
use crate::net::{ConnectionReport, NetError, NetState, PathKind};

const GOOD: Color32 = Color32::from_rgb(110, 220, 110);
const BAD: Color32 = Color32::from_rgb(240, 100, 100);
const WARN: Color32 = Color32::from_rgb(240, 200, 90);
const TEST_SECONDS_RANGE: std::ops::RangeInclusive<u32> = 1..=30;
const LOG_LINES_SHOWN: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Host,
    Join,
    Settings,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 4] = [
        (Tab::Play, "Play"),
        (Tab::Host, "Host"),
        (Tab::Join, "Join"),
        (Tab::Settings, "Settings"),
    ];
}

pub fn draw(ui: &mut Ui, tab: &mut Tab, launcher: &mut Launcher, forms: &mut NetForms) {
    egui::Panel::top("tabs").show(ui, |ui| {
        ui.horizontal(|ui| {
            for (value, label) in Tab::ALL {
                ui.selectable_value(tab, value, label);
            }
        });
    });
    egui::Panel::bottom("log").show(ui, |ui| draw_log(ui, launcher));
    egui::CentralPanel::default().show(ui, |ui| match *tab {
        Tab::Play => draw_play(ui, launcher),
        Tab::Host => draw_host(ui, forms),
        Tab::Join => draw_join(ui, forms),
        Tab::Settings => draw_settings(ui, launcher),
    });
}

fn draw_log(ui: &mut Ui, launcher: &Launcher) {
    let skip = launcher.log.len().saturating_sub(LOG_LINES_SHOWN);
    for line in launcher.log.iter().skip(skip) {
        ui.label(RichText::new(line).weak());
    }
    if launcher.log.is_empty() {
        ui.label(RichText::new("ready").weak());
    }
}

fn draw_play(ui: &mut Ui, launcher: &mut Launcher) {
    let ctx = ui.ctx().clone();
    egui::Grid::new("play-status")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Game");
            match &launcher.game {
                Some(game) => {
                    let text = format!("found: {}", game.game_dir.display());
                    let color = if game.exe_present() { GOOD } else { WARN };
                    ui.colored_label(color, text);
                }
                None => {
                    ui.colored_label(BAD, "not found (set the folder in Settings)");
                }
            }
            ui.end_row();
            ui.label("Mod");
            let (text, color) = match launcher.mod_status {
                ModStatus::NotInstalled => ("not installed", WARN),
                ModStatus::Current => ("installed, current build", GOOD),
                ModStatus::Outdated => ("installed, older build", WARN),
                ModStatus::Foreign => ("a different xinput9_1_0.dll is in the game folder", BAD),
            };
            ui.colored_label(color, text);
            ui.end_row();
            ui.label("Build");
            match &launcher.dll_source {
                Some(path) => ui.label(path.display().to_string()),
                None => ui.colored_label(WARN, "no rc_hook.dll next to the launcher"),
            };
            ui.end_row();
            ui.label("Proton");
            let prefix = launcher
                .game
                .as_ref()
                .and_then(|game| game.proton_prefix.as_ref());
            match prefix {
                Some(path) => ui.label(path.display().to_string()),
                None => ui.label(RichText::new("none").weak()),
            };
            ui.end_row();
            ui.label("Game process");
            if launcher.game_running {
                ui.colored_label(WARN, "running");
            } else {
                ui.label("not running");
            }
            ui.end_row();
        });
    ui.separator();
    let has_game = launcher.game.is_some();
    let idle = !launcher.busy();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                has_game && idle && !launcher.game_running,
                egui::Button::new("Launch"),
            )
            .clicked()
        {
            launcher.launch();
        }
        let install_label = match launcher.mod_status {
            ModStatus::NotInstalled | ModStatus::Foreign => "Install mod",
            ModStatus::Current | ModStatus::Outdated => "Update mod",
        };
        let can_change = has_game && idle && !launcher.game_running;
        if ui
            .add_enabled(can_change, egui::Button::new(install_label))
            .clicked()
        {
            launcher.start_install(&ctx);
        }
        let installed = matches!(
            launcher.mod_status,
            ModStatus::Current | ModStatus::Outdated
        );
        if ui
            .add_enabled(can_change && installed, egui::Button::new("Uninstall"))
            .clicked()
        {
            launcher.start_uninstall(&ctx);
        }
        if ui
            .add_enabled(has_game && idle, egui::Button::new("Back up saves"))
            .clicked()
        {
            launcher.backup_saves();
        }
        if ui.add_enabled(idle, egui::Button::new("Refresh")).clicked() {
            launcher.refresh();
        }
    });
    if launcher.busy() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("working...");
        });
    }
}

fn draw_host(ui: &mut Ui, forms: &mut NetForms) {
    let ctx = ui.ctx().clone();
    let status = forms.net.status();
    egui::Grid::new("host-form").num_columns(2).show(ui, |ui| {
        ui.label("Password");
        ui.add(
            egui::TextEdit::singleline(&mut forms.host_password)
                .password(true)
                .hint_text("optional"),
        );
        ui.end_row();
    });
    ui.horizontal(|ui| {
        let hosting = matches!(
            status.state,
            NetState::Hosting { .. } | NetState::Connected { .. }
        );
        if ui
            .add_enabled(
                !hosting && !forms.hosting_pending(),
                egui::Button::new("Start hosting"),
            )
            .clicked()
        {
            forms.start_host(&ctx);
        }
        if ui.add_enabled(hosting, egui::Button::new("Stop")).clicked() {
            forms.leave();
        }
    });
    if forms.hosting_pending() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("starting...");
        });
    }
    match &forms.host_result {
        Some(Err(error)) => {
            ui.colored_label(BAD, error.to_string());
        }
        Some(Ok(ticket)) => draw_ticket(ui, ticket),
        None => {}
    }
    draw_state(ui, &status.state);
}

fn draw_ticket(ui: &mut Ui, ticket: &str) {
    ui.label("Ticket (send this to your friend)");
    let mut shown = ticket.to_owned();
    ui.add(egui::TextEdit::singleline(&mut shown).desired_width(f32::INFINITY));
    if ui.button("Copy ticket").clicked() {
        ui.ctx().copy_text(ticket.to_owned());
    }
}

fn draw_join(ui: &mut Ui, forms: &mut NetForms) {
    let ctx = ui.ctx().clone();
    let status = forms.net.status();
    egui::Grid::new("join-form").num_columns(2).show(ui, |ui| {
        ui.label("Ticket");
        ui.add(egui::TextEdit::singleline(&mut forms.join_ticket).desired_width(f32::INFINITY));
        ui.end_row();
        ui.label("Password");
        ui.add(
            egui::TextEdit::singleline(&mut forms.join_password)
                .password(true)
                .hint_text("if the host set one"),
        );
        ui.end_row();
        ui.label("Test length");
        ui.add(
            egui::DragValue::new(&mut forms.test_seconds)
                .range(TEST_SECONDS_RANGE)
                .suffix(" s"),
        );
        ui.end_row();
    });
    let has_ticket = !forms.join_ticket.trim().is_empty();
    ui.horizontal(|ui| {
        let connected = matches!(status.state, NetState::Connected { .. } | NetState::Joining);
        if ui
            .add_enabled(
                has_ticket && !connected && !forms.joining_pending(),
                egui::Button::new("Join"),
            )
            .clicked()
        {
            forms.start_join(&ctx);
        }
        if ui
            .add_enabled(
                has_ticket && !forms.testing_pending(),
                egui::Button::new("Test connection"),
            )
            .clicked()
        {
            forms.start_test(&ctx);
        }
        if ui
            .add_enabled(connected, egui::Button::new("Leave"))
            .clicked()
        {
            forms.leave();
        }
    });
    if forms.joining_pending() || forms.testing_pending() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("working...");
        });
    }
    if let Some(Err(error)) = &forms.join_result {
        ui.colored_label(BAD, error.to_string());
    }
    match &forms.test_result {
        Some(Ok(report)) => draw_report(ui, report),
        Some(Err(error)) => draw_test_error(ui, error),
        None => {}
    }
    draw_state(ui, &status.state);
}

fn draw_test_error(ui: &mut Ui, error: &NetError) {
    ui.colored_label(BAD, error.to_string());
}

fn draw_report(ui: &mut Ui, report: &ConnectionReport) {
    ui.separator();
    let color = if report.reachable { GOOD } else { BAD };
    ui.colored_label(color, &report.verdict);
    let path = match report.path {
        PathKind::Direct => "direct",
        PathKind::Relay => "relayed",
        PathKind::Unknown => "unknown",
    };
    ui.label(format!("path: {path}, loss: {:.1}%", report.loss_percent));
    if let Some(rtt) = report.rtt {
        ui.label(format!(
            "round trip: min {} ms, avg {} ms, max {} ms",
            rtt.min.as_millis(),
            rtt.avg.as_millis(),
            rtt.max.as_millis()
        ));
    }
}

fn draw_state(ui: &mut Ui, state: &NetState) {
    ui.separator();
    match state {
        NetState::Idle => ui.label("Status: idle"),
        NetState::Hosting { .. } => ui.label("Status: hosting, waiting for a player"),
        NetState::Joining => ui.label("Status: joining..."),
        NetState::Connected { peer_name } => {
            ui.colored_label(GOOD, format!("Status: connected to {peer_name}"))
        }
        NetState::Failed(reason) => ui.colored_label(BAD, format!("Status: failed ({reason})")),
    };
}

fn draw_settings(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Game folder (leave empty to find it automatically)");
    ui.add(egui::TextEdit::singleline(&mut launcher.override_text).desired_width(f32::INFINITY));
    ui.horizontal(|ui| {
        if ui.button("Apply").clicked() {
            launcher.apply_override();
        }
        if ui.button("Use automatic").clicked() {
            launcher.clear_override();
        }
    });
    ui.separator();
    match &launcher.game {
        Some(game) => ui.label(format!("Using: {}", game.game_dir.display())),
        None => ui.colored_label(BAD, "No game folder found."),
    };
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
}

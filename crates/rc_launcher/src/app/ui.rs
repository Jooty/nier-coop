use eframe::egui::{self, Color32, RichText, Ui};

use crate::app::{Launcher, NetForms, Role};
use crate::install::ModStatus;
use crate::net::NetState;

const GOOD: Color32 = Color32::from_rgb(110, 220, 110);
const BAD: Color32 = Color32::from_rgb(240, 100, 100);
const WARN: Color32 = Color32::from_rgb(240, 200, 90);
const LOG_LINES_SHOWN: usize = 6;
const COPIED_NOTICE_SECONDS: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Saves,
    Setup,
}

impl Tab {
    pub const ALL: [(Tab, &'static str); 3] = [
        (Tab::Play, "Play"),
        (Tab::Saves, "Saves"),
        (Tab::Setup, "Setup"),
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
    egui::Panel::bottom("log").show(ui, |ui| {
        draw_status_line(ui, launcher, forms);
        ui.separator();
        draw_log(ui, launcher);
    });
    egui::CentralPanel::default().show(ui, |ui| match *tab {
        Tab::Play => draw_play(ui, launcher, forms),
        Tab::Saves => draw_saves(ui, launcher),
        Tab::Setup => {
            draw_setup(ui, launcher);
            ui.separator();
            draw_settings(ui, launcher);
        }
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

fn draw_setup(ui: &mut Ui, launcher: &mut Launcher) {
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
                    ui.colored_label(BAD, "not found (set the folder in Setup)");
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

fn draw_host(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
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
        let can_host = launcher.can_start_game() && !hosting && !forms.hosting_pending();
        if ui
            .add_enabled(can_host, egui::Button::new("Host"))
            .clicked()
        {
            forms.start_host(&ctx);
            launcher.launch(Role::Host);
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
}

fn draw_ticket(ui: &mut Ui, ticket: &str) {
    ui.label("Ticket (send this to your friend)");
    let mut shown = ticket;
    ui.add(egui::TextEdit::singleline(&mut shown).desired_width(f32::INFINITY));
    let copied_id = ui.make_persistent_id("ticket-copied-at");
    ui.horizontal(|ui| {
        if ui.button("Copy ticket").clicked() {
            ui.ctx().copy_text(ticket.to_owned());
            let now = ui.input(|input| input.time);
            ui.data_mut(|data| data.insert_temp(copied_id, now));
        }
        let copied_at: Option<f64> = ui.data(|data| data.get_temp(copied_id));
        if let Some(copied_at) = copied_at {
            let elapsed = ui.input(|input| input.time) - copied_at;
            if elapsed < COPIED_NOTICE_SECONDS {
                ui.colored_label(GOOD, "Copied");
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
    });
}

fn draw_join(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
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
        ui.label("Saves");
        ui.checkbox(&mut launcher.guest_keeps_saving, "Guest keeps saving");
        ui.end_row();
    });
    let has_ticket = !forms.join_ticket.trim().is_empty();
    ui.horizontal(|ui| {
        let connected = matches!(status.state, NetState::Connected { .. } | NetState::Joining);
        let can_join =
            launcher.can_start_game() && has_ticket && !connected && !forms.joining_pending();
        if ui
            .add_enabled(can_join, egui::Button::new("Join"))
            .clicked()
        {
            forms.start_join(&ctx);
        }
        if ui
            .add_enabled(connected, egui::Button::new("Leave"))
            .clicked()
        {
            forms.leave();
        }
    });
    if forms.joining_pending() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("testing the connection...");
        });
    }
    if let Some(Err(error)) = &forms.join_result {
        ui.colored_label(BAD, error.to_string());
    }
}

fn draw_status_line(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    ui.horizontal_wrapped(|ui| {
        status_item(ui, "game", launcher.game_running, "running", "not running");
        status_item(ui, "link", launcher.link.game_linked, "up", "down");
        match &state {
            NetState::Connected { peer_name } => {
                ui.colored_label(GOOD, format!("peer: {peer_name}"));
            }
            NetState::Hosting { .. } => {
                ui.colored_label(WARN, "peer: waiting");
            }
            NetState::Joining => {
                ui.colored_label(WARN, "peer: joining");
            }
            NetState::Failed(reason) => {
                ui.colored_label(BAD, format!("failed: {reason}"));
            }
            NetState::Idle => {
                ui.label("peer: none");
            }
        }
        match launcher.link.rtt {
            Some(rtt) => ui.label(format!("ping: {} ms", rtt.as_millis())),
            None => ui.label("ping: -"),
        };
    });
}

fn status_item(ui: &mut Ui, label: &str, up: bool, up_text: &str, down_text: &str) {
    if up {
        ui.colored_label(GOOD, format!("{label}: {up_text}"));
    } else {
        ui.label(format!("{label}: {down_text}"));
    }
}

fn draw_play(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    draw_slot_line(ui, launcher);
    if launcher.game.is_none() {
        ui.colored_label(BAD, "Game not found. Set the folder in Setup.");
    } else if !launcher.can_start_game() && !launcher.game_running {
        ui.colored_label(WARN, "No rc_hook.dll next to the launcher to install.");
    }
    if crate::DEV_BUILD && launcher.game_running {
        ui.colored_label(
            WARN,
            "Dev build: Host or Join starts a second game copy as a guest.",
        );
    }
    ui.separator();
    ui.heading("Host");
    draw_host(ui, launcher, forms);
    ui.separator();
    ui.heading("Join");
    draw_join(ui, launcher, forms);
}

fn draw_slot_line(ui: &mut Ui, launcher: &Launcher) {
    let picked = launcher
        .selected_slot
        .and_then(|index| launcher.slots.iter().find(|slot| slot.index == index));
    match picked {
        Some(slot) => ui.label(format!(
            "Playing slot {}: {} (level {}, {})",
            slot.index,
            slot.name,
            slot.level,
            slot.play_time_text()
        )),
        None => ui.label("No save slot picked: the game opens at its title menu."),
    };
}

fn draw_saves(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Slots (pick the one the game starts in)");
    let mut picked = launcher.selected_slot;
    egui::Grid::new("slots")
        .num_columns(5)
        .striped(true)
        .show(ui, |ui| {
            for slot in &launcher.slots {
                ui.add_enabled_ui(slot.used, |ui| {
                    ui.selectable_value(
                        &mut picked,
                        Some(slot.index),
                        format!("Slot {}", slot.index),
                    );
                });
                if slot.used {
                    ui.label(&slot.name);
                    ui.label(format!("level {}", slot.level));
                    ui.label(slot.play_time_text());
                    ui.label(&slot.phase);
                } else {
                    ui.label(RichText::new("empty").weak());
                }
                ui.end_row();
            }
        });
    launcher.selected_slot = picked;
    if launcher.slots.is_empty() {
        ui.label(RichText::new("No save file found yet.").weak());
    }
    ui.separator();
    ui.label("Backups");
    let idle = !launcher.busy();
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                launcher.game.is_some() && idle,
                egui::Button::new("Back up now"),
            )
            .clicked()
        {
            launcher.backup_saves();
        }
        let can_restore = idle && !launcher.game_running && launcher.selected_backup.is_some();
        if ui
            .add_enabled(can_restore, egui::Button::new("Restore selected"))
            .clicked()
        {
            launcher.restore_selected_backup();
        }
    });
    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for stamp in launcher.backups.clone() {
                ui.selectable_value(&mut launcher.selected_backup, Some(stamp.clone()), stamp);
            }
        });
    ui.label(
        RichText::new("Restoring first copies the current saves to a before-restore backup.")
            .weak(),
    );
}

fn draw_settings(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Game folder");
    ui.add(egui::TextEdit::singleline(&mut launcher.override_text).desired_width(f32::INFINITY));
    ui.horizontal(|ui| {
        if ui.button("Apply").clicked() {
            launcher.apply_override();
        }
        if ui.button("Find").clicked() {
            launcher.find_game();
        }
    });
    ui.separator();
    ui.label(
        "Player name (applies to the game at the next launch; the network name at the next start)",
    );
    ui.add(egui::TextEdit::singleline(&mut launcher.name_text).desired_width(f32::INFINITY));
    if ui.button("Save name").clicked() {
        launcher.apply_name();
    }
    ui.separator();
    match &launcher.game {
        Some(game) => ui.label(format!("Using: {}", game.game_dir.display())),
        None => ui.colored_label(BAD, "No game folder found."),
    };
    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
}

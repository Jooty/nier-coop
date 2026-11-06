use eframe::egui::{self, RichText, Ui};
use rc_launcher_core::{MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT};

use crate::app::style::{BAD, GOOD, WARN};
use crate::app::{Launcher, NetForms};
use crate::install::ModStatus;

pub(super) fn draw_settings(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            draw_game_folder(ui, launcher);
            ui.separator();
            draw_install_status(ui, launcher);
            ui.separator();
            draw_install_buttons(ui, launcher);
            ui.separator();
            draw_player_settings(ui, launcher, forms);
            ui.separator();
            ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        });
}

fn draw_game_folder(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label("Game folder");
    let found = launcher
        .game
        .as_ref()
        .is_some_and(|game| game.exe_present());
    let color = if found { GOOD } else { BAD };
    ui.add(
        egui::TextEdit::singleline(&mut launcher.override_text)
            .text_color(color)
            .desired_width(f32::INFINITY),
    );
    ui.horizontal(|ui| {
        if ui.button("Apply").clicked() {
            launcher.apply_override();
        }
        if ui.button("Find").clicked() {
            launcher.find_game();
        }
    });
    match &launcher.game {
        Some(game) if game.exe_present() => {
            ui.colored_label(GOOD, format!("found: {}", game.game_dir.display()));
        }
        Some(game) => {
            ui.colored_label(
                BAD,
                format!(
                    "not found: no game executable in {}",
                    game.game_dir.display()
                ),
            );
        }
        None => {
            ui.colored_label(BAD, "not found: type the folder and press Apply");
        }
    }
}

fn draw_install_status(ui: &mut Ui, launcher: &Launcher) {
    egui::Grid::new("install-status")
        .num_columns(2)
        .show(ui, |ui| {
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
}

fn draw_install_buttons(ui: &mut Ui, launcher: &mut Launcher) {
    let ctx = ui.ctx().clone();
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

fn draw_player_settings(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let mut changed = false;
    changed |= ui
        .checkbox(
            &mut launcher.config.keep_running,
            "Keep running in the background",
        )
        .on_hover_text(
            "The game keeps playing when its window loses focus (always on in dev builds)",
        )
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.event_log, "Show the event log")
        .on_hover_text("The short messages at the bottom left of the game screen")
        .changed();
    if changed {
        launcher.apply_settings();
        forms.net.set_player_settings(launcher.player_settings());
    }
    ui.separator();
    draw_host_only_settings(ui, launcher, forms);
    ui.label(
        RichText::new("These are shared with the game: change them here or in the game's menu.")
            .weak(),
    );
}

fn draw_host_only_settings(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    if launcher.link.guest {
        let options = launcher.host_options_seen.unwrap_or_default();
        ui.label(RichText::new("Set by the host:").weak());
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_names }, "Show names over players"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_party_frame }, "Show party frame"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_distance }, "Show distance"),
        );
        ui.add_enabled(
            false,
            egui::Checkbox::new(&mut { options.show_ping }, "Show ping"),
        );
        ui.add_enabled(
            false,
            egui::Slider::new(
                &mut { options.enemy_health_percent },
                0..=MAX_ENEMY_HEALTH_PERCENT,
            )
            .suffix("%")
            .text("Enemy health per extra player"),
        );
        ui.add_enabled(
            false,
            egui::Slider::new(
                &mut { options.enemy_damage_percent },
                0..=MAX_ENEMY_DAMAGE_PERCENT,
            )
            .suffix("%")
            .text("Enemy damage per extra player"),
        );
        return;
    }
    let mut changed = false;
    changed |= ui
        .checkbox(&mut launcher.config.show_names, "Show names over players")
        .on_hover_text("Host only: everyone in the session sees the host's choice")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_party_frame, "Show party frame")
        .on_hover_text("Host only: everyone in the session sees the host's choice")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_distance, "Show distance")
        .on_hover_text("Host only: adds the partner's distance to the name tag")
        .changed();
    changed |= ui
        .checkbox(&mut launcher.config.show_ping, "Show ping")
        .on_hover_text("Host only: adds the connection's ping to the name tag")
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(
                &mut launcher.config.enemy_health_percent,
                0..=MAX_ENEMY_HEALTH_PERCENT,
            )
            .suffix("%")
            .text("Enemy health per extra player"),
        )
        .on_hover_text("Host only: enemies have this much more health per extra player")
        .changed();
    changed |= ui
        .add(
            egui::Slider::new(
                &mut launcher.config.enemy_damage_percent,
                0..=MAX_ENEMY_DAMAGE_PERCENT,
            )
            .suffix("%")
            .text("Enemy damage per extra player"),
        )
        .on_hover_text("Host only: enemy hits on either player hurt this much more")
        .changed();
    if changed {
        launcher.apply_settings();
        forms.session_options = launcher.config.session_options();
        forms.push_session_options();
    }
}

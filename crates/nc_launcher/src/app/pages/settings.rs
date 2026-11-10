use eframe::egui::{self, RichText, Ui};

use crate::app::style::{BAD, GOOD, MUTED, WARN};
use crate::app::{Launcher, NetForms, UpdateState, heading_text};
use crate::game_link::START_CHOICES;
use crate::install::ModStatus;
use crate::lobby::lobby_picks_character_id;
use crate::update::{BUILD_COMMIT, short_commit};

const SAVES_COME_LATER_NOTE: &str =
    "Saves for this game come later. The launcher does not read or write its save folder.";

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
            if lobby_picks_character_id(launcher.profile) {
                ui.separator();
                draw_start_setting(ui, launcher);
            }
            ui.separator();
            draw_updates(ui, launcher);
            ui.label(format!(
                "Version {} ({})",
                env!("CARGO_PKG_VERSION"),
                short_commit(BUILD_COMMIT)
            ));
        });
}

fn draw_game_folder(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label(heading_text("Game folder"));
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
            let (text, color) = if !launcher.profile.support.install {
                ("mod not built yet".to_owned(), MUTED)
            } else {
                match launcher.mod_status {
                    ModStatus::NotInstalled => ("not installed".to_owned(), WARN),
                    ModStatus::Current => ("installed, current build".to_owned(), GOOD),
                    ModStatus::Outdated => ("installed, older build".to_owned(), WARN),
                    ModStatus::Foreign => (
                        format!(
                            "a different {} is in the game folder",
                            launcher.profile.loader_dll_name
                        ),
                        BAD,
                    ),
                }
            };
            ui.colored_label(color, text);
            ui.end_row();
            ui.label("Build");
            match &launcher.dll_source {
                Some(path) => ui.label(path.display().to_string()),
                None => ui.colored_label(
                    WARN,
                    format!("no {} next to the launcher", launcher.profile.hook_dll_name),
                ),
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
            if launcher.any_game_running {
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
    let can_install = launcher.profile.support.install;
    let updating = matches!(
        launcher.updater.state,
        UpdateState::Checking | UpdateState::Updating
    );
    let idle = !launcher.busy() && !updating;
    ui.horizontal(|ui| {
        let install_label = match launcher.mod_status {
            ModStatus::NotInstalled | ModStatus::Foreign => "Install mod",
            ModStatus::Current | ModStatus::Outdated => "Update mod",
        };
        let can_change = has_game && can_install && idle && !launcher.any_game_running;
        if ui
            .add_enabled(
                has_game && can_install && idle,
                egui::Button::new(install_label),
            )
            .clicked()
        {
            launcher.start_mod_update(&ctx);
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
            .add_enabled(
                has_game && launcher.profile.support.saves && idle,
                egui::Button::new("Back up saves"),
            )
            .clicked()
        {
            launcher.backup_saves();
        }
        if ui.add_enabled(idle, egui::Button::new("Refresh")).clicked() {
            launcher.refresh();
        }
    });
    if !launcher.profile.support.saves {
        ui.label(RichText::new(SAVES_COME_LATER_NOTE).weak());
    }
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
}

fn draw_start_setting(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label(heading_text("Start"));
    let mut start = launcher.config.start;
    ui.horizontal(|ui| {
        ui.label("Start");
        egui::ComboBox::from_id_salt("start-choice")
            .selected_text(start.choice.label())
            .show_ui(ui, |ui| {
                for choice in START_CHOICES {
                    ui.selectable_value(&mut start.choice, choice, choice.label());
                }
            });
    });
    ui.horizontal(|ui| {
        ui.label("Chapter candidate");
        ui.add(egui::DragValue::new(&mut start.index));
        ui.label("Slot");
        ui.add(egui::DragValue::new(&mut start.slot).range(0..=3));
    });
    let mut hold_story = !start.story_allowed;
    ui.checkbox(&mut hold_story, "Hold the story")
        .on_hover_text(
            "Off: the game may change scene and move the story on its own; the mod only logs it",
        );
    start.story_allowed = !hold_story;
    launcher.set_start(start);
}

fn draw_updates(ui: &mut Ui, launcher: &mut Launcher) {
    ui.label(heading_text("Updates"));
    let ctx = ui.ctx().clone();
    let default_text = launcher.update_url().unwrap_or_default();
    let mut server_text = launcher.updater.server_text.take().unwrap_or(default_text);
    ui.horizontal(|ui| {
        ui.label("Build server");
        if ui
            .add(egui::TextEdit::singleline(&mut server_text).desired_width(320.0))
            .changed()
        {
            let trimmed = server_text.trim().to_owned();
            launcher.config.update_url = (!trimmed.is_empty()).then_some(trimmed);
            launcher.apply_settings();
        }
    });
    launcher.updater.server_text = Some(server_text);
    if launcher.update_url().is_none() {
        return;
    }
    if ui
        .checkbox(
            &mut launcher.config.check_for_updates,
            "Check for updates when the launcher starts",
        )
        .changed()
    {
        launcher.apply_settings();
    }
    if launcher.log_upload_available()
        && ui
            .checkbox(
                &mut launcher.config.send_logs,
                "Send logs to the build server after each game",
            )
            .changed()
    {
        launcher.apply_settings();
    }
    ui.horizontal(|ui| {
        let checking = matches!(
            launcher.updater.state,
            UpdateState::Checking | UpdateState::Updating
        );
        if ui
            .add_enabled(!checking, egui::Button::new("Check for updates"))
            .clicked()
        {
            launcher.start_update_check(&ctx);
        }
        if let UpdateState::Available(update) = launcher.updater.state.clone()
            && ui
                .add_enabled(
                    !checking,
                    egui::Button::new(format!(
                        "Update to {}",
                        short_commit(&update.manifest.commit)
                    )),
                )
                .clicked()
        {
            launcher.start_update_apply(&ctx, false);
        }
    });
    match &launcher.updater.state {
        UpdateState::Idle => {}
        UpdateState::Checking => {
            ui.label("checking...");
        }
        UpdateState::UpToDate => {
            ui.colored_label(GOOD, "up to date");
        }
        UpdateState::Updated(short) => {
            ui.colored_label(GOOD, format!("Mod updated to {short}"));
        }
        UpdateState::Available(update) => {
            ui.colored_label(
                WARN,
                format!(
                    "update available: {}",
                    short_commit(&update.manifest.commit)
                ),
            );
        }
        UpdateState::Updating => {
            ui.label("downloading and installing...");
        }
        UpdateState::Failed(message) => {
            ui.colored_label(BAD, message);
        }
    }
    ui.separator();
}

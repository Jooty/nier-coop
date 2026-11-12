use std::path::Path;

use eframe::egui::{Color32, FontId, Id, Label, Response, RichText, Ui};

use crate::app::palette::{INK, INK_MUTED, NOT_READY};
use crate::app::parts::{
    LINE_BUTTON_HEIGHT_PX, LineTone, caps_label, copy_line, fact_line, field_line, line_button,
    section_heading,
};
use crate::app::style::{body_font, small_font};
use crate::app::{Launcher, UpdateState};
use crate::install::ModStatus;

const FOLDER_LINE_WIDTH_PX: f32 = 384.0;
const COPY_LINE_INSET_PX: f32 = 12.0;
const FOLDER_FIELD_WIDTH_PX: f32 = 360.0;
const FACTS_TOP_GAP_PX: f32 = 8.0;
const BUTTONS_TOP_GAP_PX: f32 = 16.0;
const BUTTON_GAP_PX: f32 = 14.0;
const BUTTONS_LEFT_PX: f32 = 12.0;
const FOLDER_HINT: &str = "Type the game folder";
const FOLDER_EDITOR_SALT: &str = "settings-folder-editor";

pub(super) fn draw_game_section(ui: &mut Ui, launcher: &mut Launcher) {
    section_heading(ui, "Game", None);
    ui.add_space(FACTS_TOP_GAP_PX);
    let editor_id = Id::unique(FOLDER_EDITOR_SALT);
    let found = launcher
        .game
        .as_ref()
        .is_some_and(|game| game.exe_present());
    let editing = !found || ui.data(|data| data.get_temp::<bool>(editor_id).unwrap_or(false));
    draw_folder(ui, launcher, editing);
    draw_mod_facts(ui, launcher);
    ui.add_space(BUTTONS_TOP_GAP_PX);
    if draw_buttons(ui, launcher, found) {
        ui.data_mut(|data| data.insert_temp(editor_id, !editing));
    }
}

fn draw_folder(ui: &mut Ui, launcher: &mut Launcher, editing: bool) {
    let shown = launcher.game.as_ref().map(|game| game.game_dir.clone());
    let found = launcher
        .game
        .as_ref()
        .is_some_and(|game| game.exe_present());
    match (&shown, found) {
        (Some(dir), true) => {
            let full = dir.display().to_string();
            let shown = folder_two_lines(&home_as_tilde(dir));
            fact_line(ui, "Folder", INK_MUTED, |ui| {
                ui.add_space(-COPY_LINE_INSET_PX);
                copy_line(
                    ui,
                    "settings-folder",
                    &shown,
                    &full,
                    FOLDER_LINE_WIDTH_PX,
                    small_font(),
                );
            });
        }
        (Some(dir), false) => {
            let note = format!("No game executable in {}", dir.display());
            fact_line(ui, "Folder", NOT_READY, |ui| {
                fact_text(ui, note, small_font(), NOT_READY);
            });
        }
        (None, _) => {
            fact_line(ui, "Folder", NOT_READY, |ui| {
                fact_text(
                    ui,
                    "Not found: type the folder and press Apply",
                    body_font(),
                    NOT_READY,
                );
            });
        }
    }
    if !editing {
        return;
    }
    fact_line(ui, "New folder", INK_MUTED, |ui| {
        ui.spacing_mut().item_spacing.x = BUTTON_GAP_PX;
        field_line(
            ui,
            &mut launcher.override_text,
            FOLDER_FIELD_WIDTH_PX,
            FOLDER_HINT,
            small_font(),
        );
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = BUTTON_GAP_PX;
        ui.add_space(BUTTONS_LEFT_PX);
        if line_button(
            ui,
            "Apply",
            LineTone::Plain,
            LINE_BUTTON_HEIGHT_PX,
            None,
            true,
        )
        .clicked()
        {
            launcher.apply_override();
        }
        if line_button(
            ui,
            "Find",
            LineTone::Plain,
            LINE_BUTTON_HEIGHT_PX,
            None,
            true,
        )
        .clicked()
        {
            launcher.find_game();
        }
        let idle = !launcher.busy();
        if line_button(
            ui,
            "Refresh",
            LineTone::Plain,
            LINE_BUTTON_HEIGHT_PX,
            None,
            idle,
        )
        .clicked()
        {
            launcher.refresh();
        }
    });
}

fn fact_text(ui: &mut Ui, text: impl Into<String>, font: FontId, ink: Color32) -> Response {
    ui.add(Label::new(RichText::new(text).font(font).color(ink)).truncate())
}

fn home_as_tilde(path: &Path) -> String {
    let shown = dirs::home_dir()
        .and_then(|home| {
            path.strip_prefix(home)
                .ok()
                .map(|rest| Path::new("~").join(rest))
        })
        .unwrap_or_else(|| path.to_path_buf());
    shown.display().to_string()
}

fn folder_two_lines(full: &str) -> String {
    match full.rfind(['/', '\\']) {
        Some(at) if at + 1 < full.len() => format!("{}\n{}", &full[..=at], &full[at + 1..]),
        _ => full.to_owned(),
    }
}

fn draw_mod_facts(ui: &mut Ui, launcher: &Launcher) {
    let (mod_text, mod_ink) = mod_line(launcher);
    fact_line(ui, "Mod", INK_MUTED, |ui| {
        fact_text(ui, mod_text, body_font(), mod_ink);
    });
    if let Some(ready) = ready_build(launcher) {
        fact_line(ui, "Update", NOT_READY, |ui| {
            fact_text(ui, format!("Build {ready} ready"), body_font(), NOT_READY);
        });
    }
    let (build_text, build_ink) = match &launcher.dll_source {
        Some(path) => (home_as_tilde(path), INK),
        None => (
            format!("No {} next to the launcher", launcher.profile.hook_dll_name),
            NOT_READY,
        ),
    };
    fact_line(ui, "Build", INK_MUTED, |ui| {
        fact_text(ui, build_text, small_font(), build_ink);
    });
    let proton = launcher
        .game
        .as_ref()
        .and_then(|game| game.proton_prefix.as_ref())
        .map(|path| home_as_tilde(path));
    fact_line(ui, "Proton", INK_MUTED, |ui| match proton {
        Some(path) => fact_text(ui, path, small_font(), INK),
        None => fact_text(ui, "None", body_font(), INK_MUTED),
    });
    let (process, process_ink) = process_line(launcher);
    fact_line(ui, "Process", INK_MUTED, |ui| {
        fact_text(ui, process, body_font(), process_ink);
    });
}

fn mod_line(launcher: &Launcher) -> (String, Color32) {
    if !launcher.profile.support.install {
        return ("Not built yet".to_owned(), INK_MUTED);
    }
    let build = installed_build(launcher);
    match launcher.mod_status {
        ModStatus::NotInstalled => ("Not installed".to_owned(), NOT_READY),
        ModStatus::Current => (with_build("Installed", build), INK),
        ModStatus::Outdated => (with_build("Installed, older", build), NOT_READY),
        ModStatus::Foreign => (
            format!(
                "A different {} is in the game folder",
                launcher.profile.loader_dll_name
            ),
            NOT_READY,
        ),
    }
}

fn with_build(state: &str, build: Option<&str>) -> String {
    match build {
        Some(build) => format!("{state}, build {build}"),
        None => state.to_owned(),
    }
}

fn installed_build(launcher: &Launcher) -> Option<&str> {
    launcher
        .cards
        .iter()
        .find(|card| card.profile.game_id == launcher.profile.game_id)
        .and_then(|card| card.installed_build.as_deref())
}

fn ready_build(launcher: &Launcher) -> Option<&str> {
    launcher
        .cards
        .iter()
        .find(|card| card.profile.game_id == launcher.profile.game_id)
        .and_then(|card| card.ready_build.as_deref())
}

fn process_line(launcher: &Launcher) -> (&'static str, Color32) {
    match (
        launcher.own_game_running,
        launcher.link.game_linked,
        launcher.any_game_running,
    ) {
        (true, true, _) => ("Running, linked to the launcher", INK),
        (true, false, _) => ("Running, not linked yet", NOT_READY),
        (false, _, true) => ("Another copy of the game is running", NOT_READY),
        (false, _, false) => ("Not running", INK_MUTED),
    }
}

fn draw_buttons(ui: &mut Ui, launcher: &mut Launcher, found: bool) -> bool {
    let ctx = ui.ctx().clone();
    let has_game = launcher.game.is_some();
    let can_install = launcher.profile.support.install;
    let updating = matches!(
        launcher.updater.state,
        UpdateState::Checking | UpdateState::Updating
    );
    let idle = !launcher.busy() && !updating;
    let installed = matches!(
        launcher.mod_status,
        ModStatus::Current | ModStatus::Outdated
    );
    let (install_label, install_tone) = match launcher.mod_status {
        ModStatus::NotInstalled | ModStatus::Foreign => ("Install the mod", LineTone::Plain),
        ModStatus::Outdated => ("Update the mod", LineTone::Warm),
        ModStatus::Current if ready_build(launcher).is_some() => ("Update the mod", LineTone::Warm),
        ModStatus::Current => ("Reinstall the mod", LineTone::Plain),
    };
    let mut toggle_editor = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = eframe::egui::vec2(BUTTON_GAP_PX, BUTTON_GAP_PX);
        ui.add_space(BUTTONS_LEFT_PX);
        let can_change = has_game && can_install && idle && !launcher.any_game_running;
        if line_button(
            ui,
            install_label,
            install_tone,
            LINE_BUTTON_HEIGHT_PX,
            None,
            has_game && can_install && idle,
        )
        .clicked()
        {
            launcher.start_mod_update(&ctx);
        }
        if line_button(
            ui,
            "Uninstall",
            LineTone::Plain,
            LINE_BUTTON_HEIGHT_PX,
            None,
            can_change && installed,
        )
        .clicked()
        {
            launcher.start_uninstall(&ctx);
        }
        if found
            && line_button(
                ui,
                "Change folder",
                LineTone::Plain,
                LINE_BUTTON_HEIGHT_PX,
                None,
                true,
            )
            .clicked()
        {
            toggle_editor = true;
        }
        if launcher.busy() {
            caps_label(ui, "Working", INK_MUTED);
        }
    });
    toggle_editor
}

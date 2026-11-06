use std::fmt::Display;

use hudhook::imgui::{Condition, Ui};
use rc_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::memory::InProcessMemory;
use crate::overlay::mouse_panel::{MouseDiagnostics, draw_mouse_diagnostics};

#[derive(Debug, Default)]
pub(super) struct SubmenuOpen {
    pub(super) levels: bool,
    pub(super) trainer: bool,
}

pub(super) fn draw_debug_panel(
    ui: &Ui,
    gate: &Result<VerifiedBuild, GateError>,
    game: Option<&Replicant<InProcessMemory>>,
    frame: u64,
    mouse: &MouseDiagnostics,
    submenus: &mut SubmenuOpen,
) {
    ui.window("debug")
        .position([16.0, 16.0], Condition::FirstUseEver)
        .size([340.0, 300.0], Condition::FirstUseEver)
        .build(|| {
            match gate {
                Ok(build) => {
                    ui.text("build gate: pass");
                    ui.text(format!("module base: {:#x}", build.module_base()));
                }
                Err(error) => {
                    ui.text(format!("build gate: FAIL ({error})"));
                    ui.text("module base: unknown");
                }
            }
            ui.text(format!("frame: {frame}"));
            draw_mouse_diagnostics(ui, mouse);
            ui.separator();
            draw_submenu_buttons(ui, submenus);
            ui.separator();
            let Some(game) = game else {
                ui.text("game memory not read: build gate failed");
                return;
            };
            show_stats(ui, game);
        });
}

fn show_stats(ui: &Ui, game: &Replicant<InProcessMemory>) {
    show(ui, "phase", game.current_phase());
    match game.position() {
        Ok(position) => ui.text(format!(
            "position: {:.2} {:.2} {:.2}",
            position.x, position.y, position.z
        )),
        Err(error) => ui.text(format!("position: {error}")),
    }
    show_pair(ui, "hp", game.health(), game.max_health());
    show_pair(
        ui,
        "mp",
        game.magic().map(|mp| format!("{mp:.1}")),
        game.max_magic().map(|mp| format!("{mp:.1}")),
    );
    show(ui, "level", game.level());
    show(ui, "experience", game.experience());
    show(ui, "gold", game.gold());
}

fn show<T: Display, E: Display>(ui: &Ui, label: &str, result: Result<T, E>) {
    match result {
        Ok(value) => ui.text(format!("{label}: {value}")),
        Err(error) => ui.text(format!("{label}: {error}")),
    }
}

fn show_pair<T: Display, U: Display, E: Display, F: Display>(
    ui: &Ui,
    label: &str,
    current: Result<T, E>,
    max: Result<U, F>,
) {
    let current = current.map_or_else(|error| error.to_string(), |value| value.to_string());
    let max = max.map_or_else(|error| error.to_string(), |value| value.to_string());
    ui.text(format!("{label}: {current} / {max}"));
}

fn draw_submenu_buttons(ui: &Ui, submenus: &mut SubmenuOpen) {
    if ui.button("levels") {
        submenus.levels = !submenus.levels;
    }
    ui.same_line();
    if ui.button("trainer") {
        submenus.trainer = !submenus.trainer;
    }
}

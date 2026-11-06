use hudhook::imgui::{Condition, Ui};
use rc_game::Replicant;

use crate::build_gate::{GateError, VerifiedBuild};
use crate::memory::InProcessMemory;

pub(super) fn draw_debug_panel(
    ui: &Ui,
    gate: &Result<VerifiedBuild, GateError>,
    game: Option<&Replicant<InProcessMemory>>,
    frame: u64,
) {
    ui.window("debug")
        .position([16.0, 16.0], Condition::FirstUseEver)
        .size([340.0, 230.0], Condition::FirstUseEver)
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
            ui.separator();
            let Some(game) = game else {
                ui.text("game memory not read: build gate failed");
                return;
            };
            match game.position() {
                Ok(position) => ui.text(format!(
                    "position: {:.2} {:.2} {:.2}",
                    position.x, position.y, position.z
                )),
                Err(error) => ui.text(format!("position: {error}")),
            }
            match game.health() {
                Ok(health) => ui.text(format!("hp: {health}")),
                Err(error) => ui.text(format!("hp: {error}")),
            }
            match game.magic() {
                Ok(magic) => ui.text(format!("mp: {magic:.1}")),
                Err(error) => ui.text(format!("mp: {error}")),
            }
            match game.gold() {
                Ok(gold) => ui.text(format!("gold: {gold}")),
                Err(error) => ui.text(format!("gold: {error}")),
            }
            match game.zone() {
                Ok(zone) => ui.text(format!("zone: {zone}")),
                Err(error) => ui.text(format!("zone: {error}")),
            }
        });
}

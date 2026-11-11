use hudhook::imgui::{Condition, Ui};

use crate::build_gate::{GateError, VerifiedBuild};
use crate::overlay::mouse_panel::{MouseDiagnostics, draw_mouse_diagnostics};

pub(super) fn draw_diagnostics_window(
    ui: &Ui,
    gate: &Result<VerifiedBuild, GateError>,
    frame: u64,
    mouse: &MouseDiagnostics,
    open: &mut bool,
) {
    if !*open {
        return;
    }
    ui.window("diagnostics")
        .opened(open)
        .position([372.0, 16.0], Condition::FirstUseEver)
        .size([420.0, 380.0], Condition::FirstUseEver)
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
        });
}

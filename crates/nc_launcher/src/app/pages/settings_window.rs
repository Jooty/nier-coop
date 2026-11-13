use eframe::egui::Ui;

use crate::app::Launcher;
use crate::app::parts::{ControlLook, OptionNesting, option_line, section_heading};

const HEADING_TO_OPTIONS_PX: f32 = 8.0;
const MOTION_LABEL: &str = "Animate menus";

pub(super) fn draw_window_section(ui: &mut Ui, launcher: &mut Launcher) {
    section_heading(ui, "Window", None);
    ui.add_space(HEADING_TO_OPTIONS_PX);
    if option_line(
        ui,
        MOTION_LABEL,
        &mut launcher.config.motion,
        ControlLook::Editable,
        OptionNesting::Top,
    )
    .changed()
    {
        launcher.save_config();
    }
}

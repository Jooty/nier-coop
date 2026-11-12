use eframe::egui::Ui;

use crate::app::Launcher;
use crate::app::parts::{
    ControlLook, OptionNesting, StepDirection, option_line, section_heading, stepper_line,
};
use crate::game_link::START_CHOICES;

const HEADING_TO_OPTIONS_PX: f32 = 8.0;
const LAST_START_SLOT: u8 = 3;

pub(super) fn draw_start_section(ui: &mut Ui, launcher: &mut Launcher) {
    section_heading(ui, "Start", Some("Testing"));
    ui.add_space(HEADING_TO_OPTIONS_PX);
    let mut start = launcher.config.start;
    let at = START_CHOICES
        .iter()
        .position(|choice| *choice == start.choice)
        .unwrap_or(0);
    match stepper_line(ui, "Start at", start.choice.label(), true) {
        Some(StepDirection::Back) => {
            start.choice = START_CHOICES[(at + START_CHOICES.len() - 1) % START_CHOICES.len()];
        }
        Some(StepDirection::Forward) => {
            start.choice = START_CHOICES[(at + 1) % START_CHOICES.len()];
        }
        None => {}
    }
    match stepper_line(ui, "Chapter candidate", &start.index.to_string(), true) {
        Some(StepDirection::Back) => start.index = start.index.saturating_sub(1),
        Some(StepDirection::Forward) => start.index = start.index.saturating_add(1),
        None => {}
    }
    match stepper_line(ui, "Slot", &start.slot.to_string(), true) {
        Some(StepDirection::Back) => start.slot = start.slot.saturating_sub(1),
        Some(StepDirection::Forward) => start.slot = (start.slot + 1).min(LAST_START_SLOT),
        None => {}
    }
    let mut hold_story = !start.story_allowed;
    option_line(
        ui,
        "Hold the story",
        &mut hold_story,
        ControlLook::Editable,
        OptionNesting::Top,
    )
    .on_hover_text(
        "Off: the game may change scene and move the story on its own; the mod only logs it",
    );
    start.story_allowed = !hold_story;
    launcher.set_start(start);
}

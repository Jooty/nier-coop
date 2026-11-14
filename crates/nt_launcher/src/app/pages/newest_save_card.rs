use eframe::egui::Ui;

use crate::app::palette::{INK, INK_MUTED};
use crate::app::parts::{ValueLettering, section_heading, value_line};

const NEWEST_SAVE_HEADING: &str = "Co-op save";
const NEWEST_SAVE_NOTE: &str = "Picked by the game";
const HEADING_TO_LINE_GAP_PX: f32 = 10.0;
const NEWEST_SAVE_LABEL: &str = "Each player";
const NEWEST_SAVE_VALUE: &str = "Continues their newest co-op save";

pub(super) fn draw_newest_save_card(ui: &mut Ui) {
    section_heading(ui, NEWEST_SAVE_HEADING, Some(NEWEST_SAVE_NOTE));
    ui.add_space(HEADING_TO_LINE_GAP_PX);
    value_line(
        ui,
        NEWEST_SAVE_LABEL,
        NEWEST_SAVE_VALUE,
        ValueLettering::Body,
        (INK_MUTED, INK),
    );
}

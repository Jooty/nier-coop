use imgui::Ui;
use nc_ui_style::{OPTION_TOP_LABEL_LEFT_PX, rgba_to_f32};

const NOTE_LINE_GAP_PX: f32 = 4.0;

/// A sentence the game reports, in `color`, wrapped to the width and set in from the left like a label.
pub fn note_line(ui: &Ui, text: &str, color: [u8; 4]) {
    let local = ui.cursor_pos();
    let width = ui.content_region_avail()[0];
    ui.set_cursor_pos([
        local[0] + OPTION_TOP_LABEL_LEFT_PX,
        local[1] + NOTE_LINE_GAP_PX,
    ]);
    let wrap = ui.push_text_wrap_pos_with_pos(local[0] + width);
    ui.text_colored(rgba_to_f32(color), text);
    drop(wrap);
    let after = ui.cursor_pos();
    ui.set_cursor_pos([local[0], after[1] + NOTE_LINE_GAP_PX]);
    ui.dummy([0.0, 0.0]);
}

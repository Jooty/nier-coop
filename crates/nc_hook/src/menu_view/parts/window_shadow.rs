use imgui::Ui;
use nc_ui_style::HARD_SHADOW;

use crate::menu_view::ink;

const WINDOW_SHADOW_OFFSET_PX: f32 = 5.0;

/// The hard offset shadow of the current window, drawn behind every window along its right and bottom edges.
pub fn paint_window_shadow(ui: &Ui) {
    let at = ui.window_pos();
    let size = ui.window_size();
    let right = at[0] + size[0];
    let bottom = at[1] + size[1];
    let offset = WINDOW_SHADOW_OFFSET_PX;
    let draw = ui.get_background_draw_list();
    let color = ink(HARD_SHADOW);
    draw.add_rect(
        [right, at[1] + offset],
        [right + offset, bottom + offset],
        color,
    )
    .filled(true)
    .build();
    draw.add_rect([at[0] + offset, bottom], [right, bottom + offset], color)
        .filled(true)
        .build();
}

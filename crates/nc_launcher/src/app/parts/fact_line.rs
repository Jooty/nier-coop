use eframe::egui::{Align, Color32, Layout, Sense, Ui, pos2, vec2};

use crate::app::parts::caps_galley;

const FACT_KEY_WIDTH_PX: f32 = 132.0;
const FACT_KEY_LEFT_PX: f32 = 12.0;
const FACT_LINE_MIN_HEIGHT_PX: f32 = 34.0;
const FACT_KEY_TOP_PX: f32 = 9.0;

/// One fact of a section: a spaced-capital key in a fixed column at the left and the value beside it. The key
/// sits at the top, so a value of two lines reads from its first line.
pub fn fact_line<R>(
    ui: &mut Ui,
    key: &str,
    key_ink: Color32,
    value: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let (key_rect, _) = ui.allocate_exact_size(
            vec2(FACT_KEY_WIDTH_PX, FACT_LINE_MIN_HEIGHT_PX),
            Sense::hover(),
        );
        let galley = caps_galley(ui.painter(), key, key_ink);
        ui.painter().galley(
            pos2(
                key_rect.left() + FACT_KEY_LEFT_PX,
                key_rect.top() + FACT_KEY_TOP_PX,
            ),
            galley,
            key_ink,
        );
        ui.allocate_ui_with_layout(
            vec2(ui.available_width(), FACT_LINE_MIN_HEIGHT_PX),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_min_height(FACT_LINE_MIN_HEIGHT_PX);
                value(ui)
            },
        )
        .inner
    })
    .inner
}

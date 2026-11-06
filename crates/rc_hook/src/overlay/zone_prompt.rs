use hudhook::imgui::Ui;

use crate::zone_rules::prompt_text;

const PROMPT_COLOR: [f32; 4] = [1.0, 0.92, 0.55, 1.0];
const PROMPT_SHADOW_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.85];
const PROMPT_HEIGHT_FRACTION: f32 = 0.3;
const PROMPT_SHADOW_OFFSET_PIXELS: f32 = 1.0;

pub(super) fn draw_zone_prompt(ui: &Ui) {
    let Some(text) = prompt_text() else {
        return;
    };
    let [width, height] = ui.io().display_size;
    let size = ui.calc_text_size(&text);
    let at = [(width - size[0]) * 0.5, height * PROMPT_HEIGHT_FRACTION];
    let draw_list = ui.get_background_draw_list();
    draw_list.add_text(
        [
            at[0] + PROMPT_SHADOW_OFFSET_PIXELS,
            at[1] + PROMPT_SHADOW_OFFSET_PIXELS,
        ],
        PROMPT_SHADOW_COLOR,
        &text,
    );
    draw_list.add_text(at, PROMPT_COLOR, &text);
}

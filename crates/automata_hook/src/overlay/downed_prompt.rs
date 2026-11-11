use automata_game::downed::{downed_prompt, prompt_text};
use hudhook::imgui::Ui;

use crate::net::handle;
use crate::partner_driver::downed_view;

const PARTNER_NAME_WHEN_UNKNOWN: &str = "Your partner";
const PROMPT_TOP_FRACTION_OF_THE_SCREEN: f32 = 0.7;
const PROMPT_TEXT_COLOR: [f32; 4] = [1.0, 0.35, 0.3, 1.0];
const PROMPT_SHADOW_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.85];
const PROMPT_SHADOW_OFFSET_PIXELS: f32 = 1.0;
const REVIVE_BAR_WIDTH_PIXELS: f32 = 180.0;
const REVIVE_BAR_HEIGHT_PIXELS: f32 = 10.0;
const REVIVE_BAR_GAP_PIXELS: f32 = 6.0;
const REVIVE_BAR_BORDER_PIXELS: f32 = 1.0;
const REVIVE_BAR_FILL_COLOR: [f32; 4] = [0.45, 0.92, 0.5, 0.95];
const REVIVE_BAR_BACKGROUND_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

pub(super) fn draw_downed_prompt(ui: &Ui) {
    let prompt = downed_prompt(downed_view());
    let partner_name = handle()
        .and_then(|net| net.update(|shared| shared.peer_name.clone()))
        .unwrap_or_else(|| PARTNER_NAME_WHEN_UNKNOWN.to_owned());
    let Some(text) = prompt_text(prompt, &partner_name) else {
        return;
    };
    let [width, height] = ui.io().display_size;
    let size = ui.calc_text_size(&text);
    let left = (width - size[0]) * 0.5;
    let top = height * PROMPT_TOP_FRACTION_OF_THE_SCREEN;
    let draw_list = ui.get_background_draw_list();
    draw_list.add_text(
        [
            left + PROMPT_SHADOW_OFFSET_PIXELS,
            top + PROMPT_SHADOW_OFFSET_PIXELS,
        ],
        PROMPT_SHADOW_COLOR,
        &text,
    );
    draw_list.add_text([left, top], PROMPT_TEXT_COLOR, &text);
    let Some(progress) = prompt.bar() else {
        return;
    };
    let bar_left = (width - REVIVE_BAR_WIDTH_PIXELS) * 0.5;
    let bar_top = top + size[1] + REVIVE_BAR_GAP_PIXELS;
    let bar_right = bar_left + REVIVE_BAR_WIDTH_PIXELS;
    let bar_bottom = bar_top + REVIVE_BAR_HEIGHT_PIXELS;
    draw_list
        .add_rect(
            [bar_left, bar_top],
            [bar_right, bar_bottom],
            REVIVE_BAR_BACKGROUND_COLOR,
        )
        .filled(true)
        .build();
    let inner = REVIVE_BAR_BORDER_PIXELS;
    let fill_right = bar_left + inner + (REVIVE_BAR_WIDTH_PIXELS - inner * 2.0) * progress;
    draw_list
        .add_rect(
            [bar_left + inner, bar_top + inner],
            [fill_right, bar_bottom - inner],
            REVIVE_BAR_FILL_COLOR,
        )
        .filled(true)
        .build();
}

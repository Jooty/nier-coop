use automata_game::downed::{downed_prompt, prompt_text};
use hudhook::imgui::{Condition, Ui, WindowFlags};

use crate::net::handle;
use crate::partner_driver::downed_view;

const PARTNER_NAME_WHEN_UNKNOWN: &str = "Your partner";
const PROMPT_WINDOW_NAME: &str = "##downed-prompt";
const PROMPT_TOP_FRACTION_OF_THE_SCREEN: f32 = 0.68;
const PROMPT_FONT_SCALE_AT_1080_PIXELS_HIGH: f32 = 2.0;
const SCREEN_HEIGHT_THE_FONT_SCALE_IS_FOR: f32 = 1080.0;
const PROMPT_BACKGROUND_ALPHA: f32 = 0.55;
const PROMPT_TEXT_COLOR: [f32; 4] = [1.0, 0.45, 0.4, 1.0];
const REVIVE_BAR_WIDTH_IN_TEXT_HEIGHTS: f32 = 12.0;
const REVIVE_BAR_HEIGHT_IN_TEXT_HEIGHTS: f32 = 0.5;
const REVIVE_BAR_BORDER_PIXELS: f32 = 1.0;
const REVIVE_BAR_FILL_COLOR: [f32; 4] = [0.45, 0.92, 0.5, 0.95];
const REVIVE_BAR_BACKGROUND_COLOR: [f32; 4] = [0.0, 0.0, 0.0, 0.6];

pub(super) fn draw_downed_prompt(ui: &Ui) {
    let prompt = downed_prompt(downed_view());
    let partner_name = handle()
        .and_then(|net| net.update(|shared| shared.partner_name().map(str::to_owned)))
        .unwrap_or_else(|| PARTNER_NAME_WHEN_UNKNOWN.to_owned());
    let Some(text) = prompt_text(prompt, &partner_name) else {
        return;
    };
    let [width, height] = ui.io().display_size;
    ui.window(PROMPT_WINDOW_NAME)
        .flags(
            WindowFlags::NO_DECORATION
                | WindowFlags::NO_INPUTS
                | WindowFlags::NO_NAV
                | WindowFlags::NO_SAVED_SETTINGS
                | WindowFlags::NO_FOCUS_ON_APPEARING
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        )
        .bg_alpha(PROMPT_BACKGROUND_ALPHA)
        .position(
            [width * 0.5, height * PROMPT_TOP_FRACTION_OF_THE_SCREEN],
            Condition::Always,
        )
        .position_pivot([0.5, 0.0])
        .build(|| {
            ui.set_window_font_scale(
                PROMPT_FONT_SCALE_AT_1080_PIXELS_HIGH * height
                    / SCREEN_HEIGHT_THE_FONT_SCALE_IS_FOR,
            );
            ui.text_colored(PROMPT_TEXT_COLOR, &text);
            if let Some(progress) = prompt.bar() {
                draw_revive_bar(ui, progress);
            }
        });
}

fn draw_revive_bar(ui: &Ui, progress: f32) {
    let text_height = ui.text_line_height();
    let size = [
        text_height * REVIVE_BAR_WIDTH_IN_TEXT_HEIGHTS,
        text_height * REVIVE_BAR_HEIGHT_IN_TEXT_HEIGHTS,
    ];
    let [left, top] = ui.cursor_screen_pos();
    let draw_list = ui.get_window_draw_list();
    draw_list
        .add_rect(
            [left, top],
            [left + size[0], top + size[1]],
            REVIVE_BAR_BACKGROUND_COLOR,
        )
        .filled(true)
        .build();
    let inner = REVIVE_BAR_BORDER_PIXELS;
    draw_list
        .add_rect(
            [left + inner, top + inner],
            [
                left + inner + (size[0] - inner * 2.0) * progress,
                top + size[1] - inner,
            ],
            REVIVE_BAR_FILL_COLOR,
        )
        .filled(true)
        .build();
    ui.dummy(size);
}

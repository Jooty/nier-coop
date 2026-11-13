use hudhook::imgui::{Condition, Ui, WindowFlags};

use crate::event_follow_driver::shown_skip_prompt;
use crate::net::handle;

const PARTNER_NAME_WHEN_UNKNOWN: &str = "Your partner";
const SKIP_PROMPT_WINDOW_NAME: &str = "##skip-vote-prompt";
const SKIP_PROMPT_TOP_FRACTION_OF_THE_SCREEN: f32 = 0.82;
const SKIP_PROMPT_FONT_SCALE_AT_1080_PIXELS_HIGH: f32 = 1.6;
const SCREEN_HEIGHT_THE_FONT_SCALE_IS_FOR: f32 = 1080.0;
const SKIP_PROMPT_BACKGROUND_ALPHA: f32 = 0.55;
const SKIP_PROMPT_TEXT_COLOR: [f32; 4] = [0.95, 0.92, 0.8, 1.0];

pub(super) fn draw_skip_vote_prompt(ui: &Ui) {
    let Some(prompt) = shown_skip_prompt() else {
        return;
    };
    let partner_name = handle()
        .and_then(|net| net.update(|shared| shared.partner_name().map(str::to_owned)))
        .unwrap_or_else(|| PARTNER_NAME_WHEN_UNKNOWN.to_owned());
    let text = prompt.text(&partner_name);
    let [width, height] = ui.io().display_size;
    ui.window(SKIP_PROMPT_WINDOW_NAME)
        .flags(
            WindowFlags::NO_DECORATION
                | WindowFlags::NO_INPUTS
                | WindowFlags::NO_NAV
                | WindowFlags::NO_SAVED_SETTINGS
                | WindowFlags::NO_FOCUS_ON_APPEARING
                | WindowFlags::ALWAYS_AUTO_RESIZE,
        )
        .bg_alpha(SKIP_PROMPT_BACKGROUND_ALPHA)
        .position(
            [width * 0.5, height * SKIP_PROMPT_TOP_FRACTION_OF_THE_SCREEN],
            Condition::Always,
        )
        .position_pivot([0.5, 0.0])
        .build(|| {
            ui.set_window_font_scale(
                SKIP_PROMPT_FONT_SCALE_AT_1080_PIXELS_HIGH * height
                    / SCREEN_HEIGHT_THE_FONT_SCALE_IS_FOR,
            );
            ui.text_colored(SKIP_PROMPT_TEXT_COLOR, &text);
        });
}

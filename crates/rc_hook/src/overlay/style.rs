use std::path::PathBuf;

use hudhook::imgui::{Context, StyleColor};

const WINDOW_BACKGROUND_BLACK_75_PERCENT: [f32; 4] = [0.0, 0.0, 0.0, 0.75];

pub(super) fn apply_classic_black_style(context: &mut Context) {
    context.set_ini_filename(None::<PathBuf>);
    let style = context.style_mut();
    style.use_classic_colors();
    style[StyleColor::WindowBg] = WINDOW_BACKGROUND_BLACK_75_PERCENT;
}

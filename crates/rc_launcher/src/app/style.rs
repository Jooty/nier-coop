use eframe::egui::{Color32, CornerRadius, Stroke, Style, Vec2, Visuals};
use rc_ui_style as shared;

pub const GOOD: Color32 = colour(shared::GOOD);
pub const BAD: Color32 = colour(shared::BAD);
pub const WARN: Color32 = colour(shared::WARN);
const WINDOW_BACKGROUND: Color32 = colour(shared::WINDOW_BACKGROUND);
const FRAME_BACKGROUND: Color32 = colour(shared::FRAME_BACKGROUND);
const FRAME_HOVERED: Color32 = colour(shared::FRAME_HOVERED);
const BUTTON: Color32 = colour(shared::BUTTON);
const BUTTON_HOVERED: Color32 = colour(shared::BUTTON_HOVERED);
const BUTTON_ACTIVE: Color32 = colour(shared::BUTTON_ACTIVE);
const BORDER: Color32 = colour(shared::BORDER);
const TEXT: Color32 = colour(shared::TEXT);

pub fn imgui_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = WINDOW_BACKGROUND;
    visuals.window_fill = WINDOW_BACKGROUND;
    visuals.extreme_bg_color = colour(shared::INPUT_BACKGROUND);
    visuals.faint_bg_color = colour(shared::STRIPE_BACKGROUND);
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = BUTTON;
    visuals.selection.stroke = Stroke::new(shared::BORDER_SIZE_PX, TEXT);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    let states = [
        (&mut visuals.widgets.noninteractive, WINDOW_BACKGROUND),
        (&mut visuals.widgets.inactive, BUTTON),
        (&mut visuals.widgets.hovered, BUTTON_HOVERED),
        (&mut visuals.widgets.active, BUTTON_ACTIVE),
        (&mut visuals.widgets.open, FRAME_BACKGROUND),
    ];
    for (widget, fill) in states {
        widget.corner_radius = CornerRadius::ZERO;
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, BORDER);
        widget.fg_stroke = Stroke::new(shared::BORDER_SIZE_PX, TEXT);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, FRAME_HOVERED);
    visuals
}

pub fn apply_shared_spacing(style: &mut Style) {
    let [spacing_x, spacing_y] = shared::ITEM_SPACING_PX;
    let [padding_x, padding_y] = shared::FRAME_PADDING_PX;
    style.spacing.item_spacing = Vec2::new(spacing_x, spacing_y);
    style.spacing.button_padding = Vec2::new(padding_x, padding_y);
}

const fn colour(rgba: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied_const(rgba[0], rgba[1], rgba[2], rgba[3])
}

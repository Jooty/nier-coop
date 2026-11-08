use eframe::egui::{CornerRadius, Stroke, Style, Vec2, Visuals};
use rc_ui_style as shared;

use super::palette;
pub use super::palette::{BAD, GOOD, WARN};

pub fn imgui_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = palette::BACKGROUND;
    visuals.window_fill = palette::PANEL;
    visuals.window_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::BORDER);
    visuals.extreme_bg_color = palette::INPUT;
    visuals.faint_bg_color = palette::PANEL;
    visuals.override_text_color = Some(palette::TEXT);
    visuals.hyperlink_color = palette::SELECTION;
    visuals.warn_fg_color = palette::WARN;
    visuals.error_fg_color = palette::BAD;
    visuals.selection.bg_fill = palette::SELECTION;
    visuals.selection.stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::TEXT);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    let states = [
        (
            &mut visuals.widgets.noninteractive,
            palette::PANEL,
            palette::TEXT,
        ),
        (
            &mut visuals.widgets.inactive,
            palette::ACCENT,
            palette::ACCENT_TEXT,
        ),
        (
            &mut visuals.widgets.hovered,
            palette::ACCENT_HOVERED,
            palette::ACCENT_TEXT,
        ),
        (
            &mut visuals.widgets.active,
            palette::ACCENT_HOVERED,
            palette::ACCENT_TEXT,
        ),
        (&mut visuals.widgets.open, palette::INPUT, palette::TEXT),
    ];
    for (widget, fill, text) in states {
        widget.corner_radius = CornerRadius::ZERO;
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::BORDER);
        widget.fg_stroke = Stroke::new(shared::BORDER_SIZE_PX, text);
    }
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::TEXT);
    visuals.widgets.hovered.bg_stroke =
        Stroke::new(shared::BORDER_SIZE_PX, palette::ACCENT_HOVERED);
    visuals
}

pub fn apply_shared_spacing(style: &mut Style) {
    let [spacing_x, spacing_y] = shared::ITEM_SPACING_PX;
    let [padding_x, padding_y] = shared::FRAME_PADDING_PX;
    style.spacing.item_spacing = Vec2::new(spacing_x, spacing_y);
    style.spacing.button_padding = Vec2::new(padding_x, padding_y);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visuals_use_palette_background_and_text() {
        let visuals = imgui_dark_visuals();
        assert_eq!(visuals.panel_fill, palette::BACKGROUND);
        assert_eq!(visuals.override_text_color, Some(palette::TEXT));
        assert_eq!(visuals.widgets.inactive.bg_fill, palette::ACCENT);
        assert_eq!(visuals.selection.bg_fill, palette::SELECTION);
    }
}

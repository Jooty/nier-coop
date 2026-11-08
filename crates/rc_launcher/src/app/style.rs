use std::sync::Arc;

use eframe::egui::{
    Context, CornerRadius, FontData, FontDefinitions, FontFamily, RichText, Stroke, Style, Vec2,
    Visuals,
};
use rc_ui_style as shared;

use super::palette;
pub use super::palette::{BAD, GOOD, WARN};

const HEADING_FONT_NAME: &str = "EB Garamond";
const HEADING_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/EBGaramond-Variable.ttf");
const HEADING_SIZE_PX: f32 = 18.0;

pub fn imgui_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = palette::BACKGROUND;
    visuals.window_fill = palette::SURFACE;
    visuals.window_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::LINE);
    visuals.extreme_bg_color = palette::BACKGROUND;
    visuals.faint_bg_color = palette::SURFACE;
    visuals.override_text_color = Some(palette::TEXT);
    visuals.hyperlink_color = palette::ACCENT;
    visuals.warn_fg_color = palette::WARN;
    visuals.error_fg_color = palette::BAD;
    visuals.selection.bg_fill = palette::ACTIVE_FILL;
    visuals.selection.stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::ACCENT);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    let states = [
        (&mut visuals.widgets.noninteractive, palette::SURFACE),
        (&mut visuals.widgets.inactive, palette::SURFACE),
        (&mut visuals.widgets.hovered, palette::TRACK),
        (&mut visuals.widgets.active, palette::ACTIVE_FILL),
        (&mut visuals.widgets.open, palette::TRACK),
    ];
    for (widget, fill) in states {
        widget.corner_radius = CornerRadius::ZERO;
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::LINE);
        widget.fg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::TEXT);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::ACCENT);
    visuals.widgets.active.bg_stroke = Stroke::new(shared::BORDER_SIZE_PX, palette::ACCENT);
    visuals
}

pub fn heading_font_family() -> FontFamily {
    FontFamily::Name(HEADING_FONT_NAME.into())
}

pub fn heading_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .family(heading_font_family())
        .size(HEADING_SIZE_PX)
}

pub fn install_heading_font(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        HEADING_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(HEADING_FONT_BYTES)),
    );
    fonts
        .families
        .insert(heading_font_family(), vec![HEADING_FONT_NAME.to_owned()]);
    ctx.set_fonts(fonts);
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
        assert_eq!(visuals.widgets.inactive.bg_fill, palette::SURFACE);
        assert_eq!(visuals.widgets.hovered.bg_fill, palette::TRACK);
        assert_eq!(visuals.widgets.hovered.bg_stroke.color, palette::ACCENT);
        assert_eq!(visuals.hyperlink_color, palette::ACCENT);
    }
}

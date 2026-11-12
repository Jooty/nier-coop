use std::sync::Arc;

use eframe::egui::{
    Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, RichText, Stroke, Style,
    TextStyle, Theme, ThemePreference, Vec2, Visuals,
};

use super::game_look::{GameLook, neutral_look};
use super::palette;
pub use super::palette::{ACCENT, BAD, GOOD, INFO, LINE, MUTED, WARN};

const HEADING_FONT_NAME: &str = "EB Garamond";
const HEADING_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/EBGaramond-Variable.ttf");
const HEADING_SIZE_PX: f32 = 18.0;
const BODY_FONT_NAME: &str = "Fira Sans";
const BODY_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/FiraSans-Regular.ttf");
const BODY_SIZE_PX: f32 = 14.0;
const SMALL_SIZE_PX: f32 = 13.0;
const TITLE_SIZE_PX: f32 = 24.0;
const MONOSPACE_SIZE_PX: f32 = 13.0;
const BORDER_WIDTH_PX: f32 = 1.0;
const WIDGET_CORNER_RADIUS_PX: u8 = 6;
const SURFACE_CORNER_RADIUS_PX: u8 = 8;
const ITEM_SPACING_PX: Vec2 = Vec2::new(8.0, 6.0);
const BUTTON_PADDING_PX: Vec2 = Vec2::new(12.0, 5.0);
const INTERACT_HEIGHT_PX: f32 = 26.0;

pub fn launcher_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = palette::BACKGROUND;
    visuals.window_fill = palette::SURFACE;
    visuals.window_stroke = Stroke::new(BORDER_WIDTH_PX, palette::LINE_SOFT);
    visuals.extreme_bg_color = palette::BACKGROUND;
    visuals.faint_bg_color = palette::SURFACE;
    visuals.code_bg_color = palette::TRACK;
    visuals.override_text_color = Some(palette::TEXT);
    visuals.warn_fg_color = palette::WARN;
    visuals.error_fg_color = palette::BAD;
    visuals.window_corner_radius = CornerRadius::same(SURFACE_CORNER_RADIUS_PX);
    visuals.menu_corner_radius = CornerRadius::same(WIDGET_CORNER_RADIUS_PX);
    let states = [
        (&mut visuals.widgets.noninteractive, palette::SURFACE),
        (&mut visuals.widgets.inactive, palette::TRACK),
        (&mut visuals.widgets.hovered, palette::HOVERED_FILL),
        (&mut visuals.widgets.active, palette::RAISED),
        (&mut visuals.widgets.open, palette::TRACK),
    ];
    for (widget, fill) in states {
        widget.corner_radius = CornerRadius::same(WIDGET_CORNER_RADIUS_PX);
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = Stroke::new(BORDER_WIDTH_PX, palette::LINE);
        widget.fg_stroke = Stroke::new(BORDER_WIDTH_PX, palette::TEXT);
        widget.expansion = 0.0;
    }
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(SURFACE_CORNER_RADIUS_PX);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(BORDER_WIDTH_PX, palette::LINE_SOFT);
    apply_game_accent(&mut visuals, neutral_look());
    visuals
}

pub fn apply_game_accent(visuals: &mut Visuals, look: GameLook) {
    visuals.hyperlink_color = look.accent;
    visuals.selection.bg_fill = look.accent_soft();
    visuals.selection.stroke = Stroke::new(BORDER_WIDTH_PX, look.accent);
    visuals.widgets.hovered.bg_stroke = Stroke::new(BORDER_WIDTH_PX, look.accent);
    visuals.widgets.active.bg_stroke = Stroke::new(BORDER_WIDTH_PX, look.accent);
}

pub fn heading_font_family() -> FontFamily {
    FontFamily::Name(HEADING_FONT_NAME.into())
}

pub fn heading_text(text: impl Into<String>) -> RichText {
    RichText::new(text)
        .family(heading_font_family())
        .size(HEADING_SIZE_PX)
}

pub fn title_font() -> FontId {
    FontId::new(TITLE_SIZE_PX, heading_font_family())
}

pub fn small_font() -> FontId {
    FontId::proportional(SMALL_SIZE_PX)
}

pub fn body_font() -> FontId {
    FontId::proportional(BODY_SIZE_PX)
}

pub fn apply_launcher_look(ctx: &Context) {
    install_launcher_fonts(ctx);
    ctx.set_theme(ThemePreference::Dark);
    ctx.set_visuals_of(Theme::Dark, launcher_visuals());
    ctx.all_styles_mut(apply_launcher_spacing);
}

fn install_launcher_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        HEADING_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(HEADING_FONT_BYTES)),
    );
    fonts.font_data.insert(
        BODY_FONT_NAME.to_owned(),
        Arc::new(FontData::from_static(BODY_FONT_BYTES)),
    );
    fonts
        .families
        .insert(heading_font_family(), vec![HEADING_FONT_NAME.to_owned()]);
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, BODY_FONT_NAME.to_owned());
    ctx.set_fonts(fonts);
}

fn apply_launcher_spacing(style: &mut Style) {
    style.spacing.item_spacing = ITEM_SPACING_PX;
    style.spacing.button_padding = BUTTON_PADDING_PX;
    style.spacing.interact_size.y = INTERACT_HEIGHT_PX;
    style.text_styles = [
        (TextStyle::Small, small_font()),
        (TextStyle::Body, body_font()),
        (TextStyle::Button, body_font()),
        (TextStyle::Heading, title_font()),
        (TextStyle::Monospace, FontId::monospace(MONOSPACE_SIZE_PX)),
    ]
    .into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::game_look::game_look;
    use nc_launcher_core::AUTOMATA;

    #[test]
    fn visuals_use_palette_background_and_text() {
        let visuals = launcher_visuals();
        assert_eq!(visuals.panel_fill, palette::BACKGROUND);
        assert_eq!(visuals.override_text_color, Some(palette::TEXT));
        assert_eq!(visuals.widgets.inactive.bg_fill, palette::TRACK);
        assert_eq!(visuals.widgets.hovered.bg_fill, palette::HOVERED_FILL);
        assert_eq!(visuals.widgets.hovered.bg_stroke.color, palette::ACCENT);
        assert_eq!(
            visuals.widgets.noninteractive.bg_stroke.color,
            palette::LINE_SOFT
        );
        assert_eq!(visuals.hyperlink_color, palette::ACCENT);
        assert_eq!(
            visuals.widgets.inactive.corner_radius,
            CornerRadius::same(WIDGET_CORNER_RADIUS_PX)
        );
    }

    #[test]
    fn a_game_accent_replaces_the_neutral_accent() {
        let mut visuals = launcher_visuals();
        apply_game_accent(&mut visuals, game_look(&AUTOMATA));
        assert_eq!(visuals.hyperlink_color, palette::AUTOMATA_ACCENT);
        assert_eq!(visuals.selection.stroke.color, palette::AUTOMATA_ACCENT);
    }
}

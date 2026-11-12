use eframe::egui::{
    Color32, CornerRadius, FontId, Response, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};

use crate::app::palette::{BAND, GROUND, HAIRLINE};
use crate::app::style::{body_font, garamond_family};

pub(super) const BIG_BUTTON_HEIGHT_PX: f32 = 48.0;
const BIG_BUTTON_CORNER_RADIUS_PX: u8 = 8;
const PLAY_LABEL: &str = "Play";
const PLAY_FONT_SIZE_PX: f32 = 23.0;
const PLAY_LEFT_PADDING_PX: f32 = 22.0;
const PLAY_RIGHT_PADDING_PX: f32 = 26.0;
const PLAY_ICON_WIDTH_PX: f32 = 11.0;
const PLAY_ICON_HEIGHT_PX: f32 = 13.0;
const PLAY_ICON_GAP_PX: f32 = 12.0;
const PLAY_HOVER_LIGHTEN: f32 = 1.12;
const DISABLED_TEXT: Color32 = Color32::from_rgb(0x6f, 0x6c, 0x64);
const OUTLINE_SIDE_PADDING_PX: f32 = 18.0;
const OUTLINE_BORDER_ALPHA: f32 = 0.6;
const OUTLINE_FILL_ALPHA: f32 = 0.08;
const OUTLINE_HOVER_FILL_ALPHA: f32 = 0.16;

pub(super) fn play_button(ui: &mut Ui, accent: Color32, enabled: bool) -> Response {
    let font = FontId::new(PLAY_FONT_SIZE_PX, garamond_family());
    let galley = ui
        .painter()
        .layout_no_wrap(PLAY_LABEL.to_owned(), font, GROUND);
    let width = PLAY_LEFT_PADDING_PX
        + PLAY_ICON_WIDTH_PX
        + PLAY_ICON_GAP_PX
        + galley.size().x
        + PLAY_RIGHT_PADDING_PX;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, BIG_BUTTON_HEIGHT_PX), sense);
    let (fill, border, ink) = match (enabled, response.hovered()) {
        (false, _) => (BAND, HAIRLINE, DISABLED_TEXT),
        (true, true) => (lighten(accent), lighten(accent), GROUND),
        (true, false) => (accent, accent, GROUND),
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(BIG_BUTTON_CORNER_RADIUS_PX),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    let icon_left = rect.left() + PLAY_LEFT_PADDING_PX;
    let middle = rect.center().y;
    painter.add(Shape::convex_polygon(
        vec![
            pos2(icon_left, middle - PLAY_ICON_HEIGHT_PX / 2.0),
            pos2(icon_left + PLAY_ICON_WIDTH_PX, middle),
            pos2(icon_left, middle + PLAY_ICON_HEIGHT_PX / 2.0),
        ],
        ink,
        Stroke::NONE,
    ));
    let text_left = icon_left + PLAY_ICON_WIDTH_PX + PLAY_ICON_GAP_PX;
    painter.galley_with_override_text_color(
        pos2(text_left, middle - galley.size().y / 2.0),
        galley,
        ink,
    );
    response
}

pub(super) fn outline_button(ui: &mut Ui, label: &str, color: Color32, enabled: bool) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), body_font(), color);
    let width = galley.size().x + 2.0 * OUTLINE_SIDE_PADDING_PX;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, BIG_BUTTON_HEIGHT_PX), sense);
    let (fill, border, ink) = match (enabled, response.hovered()) {
        (false, _) => (Color32::TRANSPARENT, HAIRLINE, DISABLED_TEXT),
        (true, true) => (color.gamma_multiply(OUTLINE_HOVER_FILL_ALPHA), color, color),
        (true, false) => (
            color.gamma_multiply(OUTLINE_FILL_ALPHA),
            color.gamma_multiply(OUTLINE_BORDER_ALPHA),
            color,
        ),
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(BIG_BUTTON_CORNER_RADIUS_PX),
        fill,
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    painter.galley_with_override_text_color(rect.center() - galley.size() / 2.0, galley, ink);
    response
}

fn lighten(color: Color32) -> Color32 {
    let [red, green, blue, alpha] = color.to_array();
    let lift = |channel: u8| (f32::from(channel) * PLAY_HOVER_LIGHTEN).min(255.0) as u8;
    Color32::from_rgba_premultiplied(lift(red), lift(green), lift(blue), alpha)
}

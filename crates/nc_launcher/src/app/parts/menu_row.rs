use eframe::egui::{Align2, Response, Sense, Ui, pos2, vec2};

use crate::app::palette::{
    HARD_SHADOW, HATCH_LINE, HOVER_WASH, INK, INK_ON_SAND, INK_UNAVAILABLE, SAND,
};
use crate::app::parts::{caps_galley, paint_hatching, paint_square};
use crate::app::style::body_font;

const MENU_ROW_HEIGHT_PX: f32 = 34.0;
const MENU_ROW_LEFT_PADDING_PX: f32 = 28.0;
const MENU_ROW_RIGHT_PADDING_PX: f32 = 12.0;
const MENU_ROW_SQUARE_CENTRE_X_PX: f32 = 14.5;
const MENU_ROW_SQUARE_SIDE_PX: f32 = 7.0;
const MENU_ROW_SHADOW_OFFSET_PX: f32 = 3.0;

/// What a menu row's value at the right reads like.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuValue<'a> {
    None,
    Plain(&'a str),
    Capitals(&'a str),
}

/// One line of a menu (modes, saves). The selected row is inverted to a sand bar with dark ink and a hard
/// offset shadow; an unavailable row is hatched and greyed and cannot be picked.
pub fn menu_row(
    ui: &mut Ui,
    label: &str,
    value: MenuValue,
    selected: bool,
    available: bool,
) -> Response {
    let sense = if available {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), MENU_ROW_HEIGHT_PX), sense);
    let painter = ui.painter();
    let ink = match (available, selected) {
        (false, _) => INK_UNAVAILABLE,
        (true, true) => INK_ON_SAND,
        (true, false) => INK,
    };
    if !available {
        paint_hatching(painter, rect, HATCH_LINE);
    } else if selected {
        painter.rect_filled(
            rect.translate(vec2(MENU_ROW_SHADOW_OFFSET_PX, MENU_ROW_SHADOW_OFFSET_PX)),
            0.0,
            HARD_SHADOW,
        );
        painter.rect_filled(rect, 0.0, SAND);
        paint_square(
            painter,
            pos2(rect.left() + MENU_ROW_SQUARE_CENTRE_X_PX, rect.center().y),
            MENU_ROW_SQUARE_SIDE_PX,
            INK_ON_SAND,
        );
    } else if response.hovered() {
        painter.rect_filled(rect, 0.0, HOVER_WASH);
    }
    painter.text(
        pos2(rect.left() + MENU_ROW_LEFT_PADDING_PX, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        body_font(),
        ink,
    );
    let right = pos2(rect.right() - MENU_ROW_RIGHT_PADDING_PX, rect.center().y);
    match value {
        MenuValue::None => {}
        MenuValue::Plain(text) => {
            painter.text(right, Align2::RIGHT_CENTER, text, body_font(), ink);
        }
        MenuValue::Capitals(text) => {
            let galley = caps_galley(painter, text, ink);
            let at = Align2::RIGHT_CENTER.anchor_size(right, galley.size()).min;
            painter.galley(at, galley, ink);
        }
    }
    response
}

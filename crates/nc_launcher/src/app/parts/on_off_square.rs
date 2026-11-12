use eframe::egui::{Response, Sense, Ui, pos2, vec2};

use crate::app::palette::{INK, INK_MUTED, INK_UNAVAILABLE};
use crate::app::parts::{ControlLook, caps_galley, paint_open_square, paint_square};

const ON_OFF_SQUARE_SIDE_PX: f32 = 9.0;
const ON_OFF_TEXT_TO_SQUARE_PX: f32 = 10.0;
const ON_OFF_HEIGHT_PX: f32 = 20.0;

/// "ON" with a filled square or "OFF" with an open one. Clicking flips `on` when the look is editable; a
/// read-only switch is muted and an unavailable one greyed, and neither reacts.
pub fn on_off_square(ui: &mut Ui, on: &mut bool, look: ControlLook) -> Response {
    let enabled = look.editable();
    let ink = match (look, *on) {
        (ControlLook::Unavailable, _) => INK_UNAVAILABLE,
        (ControlLook::ReadOnly, _) | (ControlLook::Editable, false) => INK_MUTED,
        (ControlLook::Editable, true) => INK,
    };
    let word = if *on { "On" } else { "Off" };
    let galley = caps_galley(ui.painter(), word, ink);
    let width = galley.size().x + ON_OFF_TEXT_TO_SQUARE_PX + ON_OFF_SQUARE_SIDE_PX;
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, mut response) = ui.allocate_exact_size(vec2(width, ON_OFF_HEIGHT_PX), sense);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let painter = ui.painter();
    let middle = rect.center().y;
    painter.galley(
        pos2(rect.left(), middle - galley.size().y / 2.0),
        galley,
        ink,
    );
    let square_centre = pos2(rect.right() - ON_OFF_SQUARE_SIDE_PX / 2.0, middle);
    if *on {
        paint_square(painter, square_centre, ON_OFF_SQUARE_SIDE_PX, ink);
    } else {
        paint_open_square(painter, square_centre, ON_OFF_SQUARE_SIDE_PX, ink);
    }
    response
}

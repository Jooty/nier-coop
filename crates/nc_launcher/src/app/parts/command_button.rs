use eframe::egui::{FontId, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::palette::{
    HARD_SHADOW, HATCH_LINE, INK, INK_MUTED, INK_ON_SAND, INK_UNAVAILABLE, SAND,
};
use crate::app::parts::{paint_hatching, paint_square, spaced_job};
use crate::app::style::medium_family;

const COMMAND_LABEL_SIZE_PX: f32 = 15.0;
const COMMAND_LETTER_SPACING_EM: f32 = 0.32;
const COMMAND_LEFT_PADDING_PX: f32 = 22.0;
const COMMAND_SQUARE_SIDE_PX: f32 = 9.0;
const COMMAND_SQUARE_TO_TEXT_PX: f32 = 14.0;
const COMMAND_SHADOW_OFFSET_PX: f32 = 5.0;

/// A main command such as PLAY or START: a sand block with dark ink and a hard offset shadow when `inverted`,
/// an outline otherwise. Hovering inverts it; a disabled one is hatched and greyed.
pub fn command_button(
    ui: &mut Ui,
    label: &str,
    size: (f32, f32),
    inverted: bool,
    enabled: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(size.0, size.1), sense);
    let painter = ui.painter();
    let filled = enabled && (inverted || response.hovered());
    let ink = if !enabled {
        INK_UNAVAILABLE
    } else if filled {
        INK_ON_SAND
    } else {
        INK
    };
    if filled {
        painter.rect_filled(
            rect.translate(vec2(COMMAND_SHADOW_OFFSET_PX, COMMAND_SHADOW_OFFSET_PX)),
            0.0,
            HARD_SHADOW,
        );
        painter.rect_filled(rect, 0.0, SAND);
    } else {
        if !enabled {
            paint_hatching(painter, rect, HATCH_LINE);
        }
        let border = if enabled { INK_MUTED } else { INK_UNAVAILABLE };
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, border), StrokeKind::Inside);
    }
    let square_x = rect.left() + COMMAND_LEFT_PADDING_PX + COMMAND_SQUARE_SIDE_PX / 2.0;
    paint_square(
        painter,
        pos2(square_x, rect.center().y),
        COMMAND_SQUARE_SIDE_PX,
        ink,
    );
    let galley = painter.layout_job(spaced_job(
        &label.to_uppercase(),
        FontId::new(COMMAND_LABEL_SIZE_PX, medium_family()),
        ink,
        COMMAND_LETTER_SPACING_EM,
    ));
    let text_left =
        rect.left() + COMMAND_LEFT_PADDING_PX + COMMAND_SQUARE_SIDE_PX + COMMAND_SQUARE_TO_TEXT_PX;
    painter.galley(
        pos2(text_left, rect.center().y - galley.size().y / 2.0),
        galley,
        ink,
    );
    response
}

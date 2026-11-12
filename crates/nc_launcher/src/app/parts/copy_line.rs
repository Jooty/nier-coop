use eframe::egui::text::LayoutJob;
use eframe::egui::{
    Color32, FontId, Id, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextFormat, Ui,
    pos2, vec2,
};
use nc_launcher_core::{COPIED_HOLD_SECONDS, copied_amount};

use crate::app::palette::{
    GROUND, HAIRLINE_STRONG, HARD_SHADOW, HOVER_WASH, INK, INK_MUTED, INK_ON_SAND, SAND,
};
use crate::app::parts::{caps_galley, paint_square};

const COPY_LINE_SIDE_PADDING_PX: f32 = 12.0;
const COPY_LINE_TOP_PADDING_PX: f32 = 6.0;
const COPY_LINE_BOTTOM_PADDING_PX: f32 = 5.0;
const COPY_LINE_TEXT_TO_MARK_PX: f32 = 16.0;
const COPY_MARK_BOX_PX: f32 = 13.0;
const COPY_MARK_SQUARE_PX: f32 = 9.0;
const COPY_MARK_OFFSET_PX: f32 = 4.0;
const COPIED_SQUARE_SIDE_PX: f32 = 7.0;
const COPIED_SQUARE_TO_TEXT_PX: f32 = 9.0;
const COPIED_SHADOW_OFFSET_PX: f32 = 4.0;
const COPIED_LABEL: &str = "Copied";
const COPIED_LABEL_SHOWS_FROM: f32 = 0.5;

/// A line of text that is its own copy control: underlined at rest, outlined on hover, and on click it copies
/// `copy_value`, inverts and reads COPIED for a moment, then eases back. `text` may hold line breaks.
pub fn copy_line(
    ui: &mut Ui,
    id_salt: &str,
    text: &str,
    copy_value: &str,
    width: f32,
    font: FontId,
) -> Response {
    let id = copy_line_id(id_salt);
    let text_width =
        width - 2.0 * COPY_LINE_SIDE_PADDING_PX - COPY_LINE_TEXT_TO_MARK_PX - COPY_MARK_BOX_PX;
    let now = ui.input(|input| input.time);
    let copied_at: Option<f64> = ui.data(|data| data.get_temp(id));
    let amount = copied_at.map_or(0.0, |at| copied_amount((now - at) as f32));
    let ink = INK.lerp_to_gamma(INK_ON_SAND, amount);
    let galley = ui
        .painter()
        .layout_job(wrapped_job(text, font, ink, text_width));
    let height = galley.size().y + COPY_LINE_TOP_PADDING_PX + COPY_LINE_BOTTOM_PADDING_PX;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let clicked = response.clicked();
    if clicked {
        ui.ctx().copy_text(copy_value.to_owned());
        ui.data_mut(|data| data.insert_temp(id, now));
    }
    let hovered = response.hovered();
    if let Some(at) = copied_at
        && amount > 0.0
    {
        // Sleep through the hold, then repaint every frame while it eases back.
        let hold_left = COPIED_HOLD_SECONDS - (now - at) as f32;
        ui.ctx().request_repaint_after_secs(hold_left.max(0.0));
    }
    let painter = ui.painter();
    paint_frame(painter, rect, hovered, amount);
    painter.galley(
        pos2(
            rect.left() + COPY_LINE_SIDE_PADDING_PX,
            rect.top() + COPY_LINE_TOP_PADDING_PX,
        ),
        galley,
        ink,
    );
    let right_middle = pos2(rect.right() - COPY_LINE_SIDE_PADDING_PX, rect.center().y);
    if amount >= COPIED_LABEL_SHOWS_FROM {
        paint_copied_label(painter, right_middle);
    } else {
        let mark = if hovered { INK } else { INK_MUTED };
        paint_copy_mark(painter, right_middle, mark, amount);
    }
    response
}

/// Where a copy line keeps the time of its last click.
pub fn copy_line_id(id_salt: &str) -> Id {
    Id::new(("copy-line", id_salt))
}

fn wrapped_job(text: &str, font: FontId, color: Color32, width: f32) -> LayoutJob {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat {
            font_id: font,
            color,
            ..TextFormat::default()
        },
    );
    job.wrap.max_width = width;
    job
}

fn paint_frame(painter: &Painter, rect: Rect, hovered: bool, amount: f32) {
    if amount > 0.0 {
        painter.rect_filled(
            rect.translate(vec2(COPIED_SHADOW_OFFSET_PX, COPIED_SHADOW_OFFSET_PX)),
            0.0,
            HARD_SHADOW.gamma_multiply(amount),
        );
        painter.rect_filled(rect, 0.0, GROUND.lerp_to_gamma(SAND, amount));
        return;
    }
    if hovered {
        painter.rect_filled(rect, 0.0, HOVER_WASH);
        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, INK), StrokeKind::Inside);
    } else {
        painter.hline(
            rect.x_range(),
            rect.bottom() - 0.5,
            Stroke::new(1.0, HAIRLINE_STRONG),
        );
    }
}

fn paint_copy_mark(painter: &Painter, right_middle: Pos2, color: Color32, amount: f32) {
    let color = color.lerp_to_gamma(INK_ON_SAND, amount);
    let back = Rect::from_min_size(
        pos2(
            right_middle.x - COPY_MARK_BOX_PX,
            right_middle.y - COPY_MARK_BOX_PX / 2.0,
        ),
        vec2(COPY_MARK_SQUARE_PX, COPY_MARK_SQUARE_PX),
    );
    painter.rect_stroke(back, 0.0, Stroke::new(1.0, color), StrokeKind::Inside);
    let front = back.translate(vec2(COPY_MARK_OFFSET_PX, COPY_MARK_OFFSET_PX));
    // A 1 px ground-coloured gap separates the front square from the outline behind it.
    painter.rect_filled(front.expand(1.0), 0.0, GROUND);
    painter.rect_filled(front, 0.0, color);
}

fn paint_copied_label(painter: &Painter, right_middle: Pos2) {
    let galley = caps_galley(painter, COPIED_LABEL, INK_ON_SAND);
    let text_left = right_middle.x - galley.size().x;
    painter.galley(
        pos2(text_left, right_middle.y - galley.size().y / 2.0),
        galley,
        INK_ON_SAND,
    );
    paint_square(
        painter,
        pos2(
            text_left - COPIED_SQUARE_TO_TEXT_PX - COPIED_SQUARE_SIDE_PX / 2.0,
            right_middle.y,
        ),
        COPIED_SQUARE_SIDE_PX,
        INK_ON_SAND,
    );
}

use eframe::egui::{Align2, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::app::pages::status_ink::status_ink;
use crate::app::palette::{HAIRLINE, INK, INK_MUTED, INK_UNAVAILABLE};
use crate::app::parts::{caps_galley, paint_triangle_left, paint_triangle_right};
use crate::app::style::body_font;
use crate::lobby::PlayerStatus;

pub(super) const LEDGER_ROW_HEIGHT_PX: f32 = 32.0;
const LEDGER_SIDE_PADDING_PX: f32 = 14.0;
const STEPPER_GAP_PX: f32 = 12.0;
const STEPPER_ARROW_WIDTH_PX: f32 = 7.0;
const STEPPER_HIT_SLOP_PX: f32 = 6.0;
const LOCAL_PING_TEXT: &str = "Local";
const NOTHING_YET_TEXT: &str = "-";

/// What a seat's CHARACTER line shows: a stepper for your own pick, or a plain label.
pub(super) enum CharacterCell {
    Stepper(&'static str),
    Fixed(String),
    Empty,
}

pub(super) enum PingCell {
    Local,
    Millis(u32),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Back,
    Forward,
}

/// The facts under a seat: CHARACTER, STATUS and PING.
pub(super) struct SeatLedger {
    pub character: CharacterCell,
    pub status: Option<PlayerStatus>,
    pub ping: PingCell,
}

pub(super) fn draw_seat_ledger(ui: &mut Ui, rect: Rect, ledger: &SeatLedger) -> Option<Step> {
    let mut step = None;
    let keys = ["Character", "Status", "Ping"];
    for (index, key) in keys.into_iter().enumerate() {
        let row = Rect::from_min_size(
            pos2(
                rect.left(),
                rect.top() + index as f32 * LEDGER_ROW_HEIGHT_PX,
            ),
            vec2(rect.width(), LEDGER_ROW_HEIGHT_PX),
        );
        paint_row_frame(ui, row, index + 1 == keys.len());
        let key_galley = caps_galley(ui.painter(), key, INK_MUTED);
        ui.painter().galley(
            pos2(
                row.left() + LEDGER_SIDE_PADDING_PX,
                row.center().y - key_galley.size().y / 2.0,
            ),
            key_galley,
            INK_MUTED,
        );
        let value_right = pos2(row.right() - LEDGER_SIDE_PADDING_PX, row.center().y);
        match index {
            0 => step = draw_character(ui, value_right, &ledger.character),
            1 => draw_status(ui, value_right, ledger.status),
            _ => draw_ping(ui, value_right, &ledger.ping),
        }
    }
    step
}

fn paint_row_frame(ui: &Ui, row: Rect, last: bool) {
    let stroke = Stroke::new(1.0, HAIRLINE);
    ui.painter().hline(row.x_range(), row.top() + 0.5, stroke);
    if last {
        ui.painter()
            .hline(row.x_range(), row.bottom() - 0.5, stroke);
    }
}

fn draw_character(ui: &mut Ui, right: Pos2, cell: &CharacterCell) -> Option<Step> {
    let label = match cell {
        CharacterCell::Stepper(label) => *label,
        CharacterCell::Fixed(label) => label.as_str(),
        CharacterCell::Empty => {
            paint_plain(ui, right, NOTHING_YET_TEXT, INK_UNAVAILABLE);
            return None;
        }
    };
    if !matches!(cell, CharacterCell::Stepper(_)) {
        paint_plain(ui, right, label, INK);
        return None;
    }
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), body_font(), INK);
    let forward_tip = right;
    let text_right = forward_tip.x - STEPPER_ARROW_WIDTH_PX - STEPPER_GAP_PX;
    let back_tip = pos2(
        text_right - galley.size().x - STEPPER_GAP_PX - STEPPER_ARROW_WIDTH_PX,
        right.y,
    );
    let back = arrow_response(ui, back_tip.x, right.y, "seat-step-back");
    let forward = arrow_response(
        ui,
        forward_tip.x - STEPPER_ARROW_WIDTH_PX,
        right.y,
        "seat-step-forward",
    );
    let painter = ui.painter();
    let arrow_ink = |hovered: bool| if hovered { INK } else { INK_MUTED };
    paint_triangle_left(painter, back_tip, arrow_ink(back.hovered()));
    paint_triangle_right(painter, forward_tip, arrow_ink(forward.hovered()));
    painter.galley(
        pos2(
            text_right - galley.size().x,
            right.y - galley.size().y / 2.0,
        ),
        galley,
        INK,
    );
    if back.clicked() {
        Some(Step::Back)
    } else if forward.clicked() {
        Some(Step::Forward)
    } else {
        None
    }
}

fn arrow_response(ui: &mut Ui, left: f32, middle: f32, salt: &str) -> Response {
    let rect = Rect::from_min_size(
        pos2(left, middle - LEDGER_ROW_HEIGHT_PX / 2.0),
        vec2(STEPPER_ARROW_WIDTH_PX, LEDGER_ROW_HEIGHT_PX),
    )
    .expand2(vec2(STEPPER_HIT_SLOP_PX, 0.0));
    ui.interact(rect, ui.scope_id().with(salt), Sense::click())
}

fn draw_status(ui: &Ui, right: Pos2, status: Option<PlayerStatus>) {
    let Some(status) = status else {
        paint_plain(ui, right, NOTHING_YET_TEXT, INK_UNAVAILABLE);
        return;
    };
    paint_caps(ui, right, status.text(), status_ink(status, false));
}

fn draw_ping(ui: &Ui, right: Pos2, ping: &PingCell) {
    match ping {
        PingCell::Local => paint_caps(ui, right, LOCAL_PING_TEXT, INK_MUTED),
        PingCell::Millis(millis) => paint_plain(ui, right, &format!("{millis} ms"), INK),
        PingCell::Unknown => paint_plain(ui, right, NOTHING_YET_TEXT, INK_UNAVAILABLE),
    }
}

fn paint_caps(ui: &Ui, right: Pos2, text: &str, ink: Color32) {
    let galley = caps_galley(ui.painter(), text, ink);
    let at = Align2::RIGHT_CENTER.anchor_size(right, galley.size()).min;
    ui.painter().galley(at, galley, ink);
}

fn paint_plain(ui: &Ui, right: Pos2, text: &str, ink: Color32) {
    ui.painter()
        .text(right, Align2::RIGHT_CENTER, text, body_font(), ink);
}

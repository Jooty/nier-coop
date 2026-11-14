use std::sync::Arc;

use eframe::egui::{
    Align2, Color32, Frame, Galley, Margin, Painter, Rangef, Rect, Sense, Stroke, TextEdit, Ui,
    pos2, vec2,
};

use crate::app::pages::roster_grid::RosterGrid;
use crate::app::pages::seat::SeatName;
use crate::app::pages::seat_ledger::PingCell;
use crate::app::pages::status_ink::status_ink;
use crate::app::palette::{
    HAIRLINE, HARD_SHADOW, HATCH_LINE, INK, INK_MUTED, INK_MUTED_ON_SAND, INK_ON_SAND,
    INK_UNAVAILABLE, SAND,
};
use crate::app::parts::{paint_hatching, spaced_job};
use crate::app::style::{body_font, caps_font};
use crate::lobby::PlayerStatus;

pub(super) const ROSTER_HEADER_HEIGHT_PX: f32 = 28.0;
pub(super) const ROSTER_ROW_HEIGHT_PX: f32 = 36.0;
const ROSTER_CAPS_SPACING_EM: f32 = 0.1;
const CURSOR_SHADOW_OFFSET_PX: f32 = 3.0;
const NAME_TO_TAG_GAP_PX: f32 = 9.0;
const NAME_RIGHT_PADDING_PX: f32 = 10.0;
const NAME_UNDERLINE_TAIL_PX: f32 = 12.0;
const NAME_UNDERLINE_BELOW_CENTRE_PX: f32 = 10.5;
const REMOVE_PADDING_PX: (f32, f32) = (8.0, 3.0);
const REMOVE_TO_CHARACTER_GAP_PX: f32 = 12.0;
const NUMBER_KEY: &str = "No.";
const NAME_KEY: &str = "Name";
const ROLE_KEY: &str = "Role";
const STATUS_KEY: &str = "Status";
const PING_KEY: &str = "Ping";
const OPEN_SEAT_TEXT: &str = "Open";
const YOU_TAG: &str = "You";
const REMOVE_LABEL: &str = "Remove";
const LOCAL_PING_TEXT: &str = "Local";
const NOTHING_YET_TEXT: &str = "-";

/// One taken seat of the free roam board.
pub(super) struct RosterLine<'a> {
    pub name: SeatName<'a>,
    pub you: bool,
    pub role: &'static str,
    pub character: String,
    pub status: PlayerStatus,
    pub ping: PingCell,
    pub removable: bool,
}

/// How a taken row is drawn: at rest, or inverted to sand under the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowLook {
    Rest,
    Cursor,
}

pub(super) fn paint_roster_header(ui: &Ui, rect: Rect, grid: &RosterGrid) {
    let painter = ui.painter();
    let middle = rect.center().y;
    let left_keys = [
        (grid.number, NUMBER_KEY),
        (grid.name, NAME_KEY),
        (grid.role, ROLE_KEY),
        (grid.character, grid.character_key),
        (grid.status, STATUS_KEY),
    ];
    for (cell, key) in left_keys {
        paint_left(
            painter,
            cell,
            middle,
            roster_caps(painter, key, INK_MUTED),
            INK_MUTED,
        );
    }
    paint_right(
        painter,
        grid.ping.max,
        middle,
        roster_caps(painter, PING_KEY, INK_MUTED),
        INK_MUTED,
    );
}

pub(super) fn paint_open_row(ui: &Ui, rect: Rect, grid: &RosterGrid, number: usize, last: bool) {
    let painter = ui.painter();
    paint_hatching(painter, rect, HATCH_LINE);
    paint_row_lines(painter, rect, last);
    let middle = rect.center().y;
    let number_galley = painter.layout_no_wrap(format!("{number:02}"), body_font(), INK_MUTED);
    paint_left(painter, grid.number, middle, number_galley, INK_MUTED);
    paint_left(
        painter,
        grid.name,
        middle,
        roster_caps(painter, OPEN_SEAT_TEXT, INK_UNAVAILABLE),
        INK_UNAVAILABLE,
    );
}

/// Draws one taken row and says whether its REMOVE was clicked.
pub(super) fn draw_roster_row(
    ui: &mut Ui,
    rect: Rect,
    grid: &RosterGrid,
    number: usize,
    line: RosterLine,
    look: RowLook,
    last: bool,
) -> bool {
    let cursor = look == RowLook::Cursor;
    let (ink, muted) = if cursor {
        (INK_ON_SAND, INK_MUTED_ON_SAND)
    } else {
        (INK, INK_MUTED)
    };
    {
        let painter = ui.painter();
        if cursor {
            painter.rect_filled(
                rect.translate(vec2(CURSOR_SHADOW_OFFSET_PX, CURSOR_SHADOW_OFFSET_PX)),
                0.0,
                HARD_SHADOW,
            );
            painter.rect_filled(rect, 0.0, SAND);
        } else {
            paint_row_lines(painter, rect, last);
        }
    }
    let middle = rect.center().y;
    let show_remove = cursor && line.removable;
    let remove = show_remove.then(|| remove_rect(ui.painter(), grid, middle));
    let painter = ui.painter().clone();
    let number_galley = painter.layout_no_wrap(format!("{number:02}"), body_font(), muted);
    paint_left(&painter, grid.number, middle, number_galley, muted);
    let name_end = remove
        .as_ref()
        .map_or(grid.name.max - NAME_RIGHT_PADDING_PX, |(button, _)| {
            button.left() - NAME_RIGHT_PADDING_PX
        });
    let name_cell = Rangef::new(grid.name.min, name_end.max(grid.name.min));
    draw_name(ui, name_cell, middle, line.name, line.you, (ink, muted));
    if !show_remove {
        paint_left(
            &painter,
            grid.role,
            middle,
            roster_caps(&painter, line.role, muted),
            muted,
        );
    }
    let character = painter.layout_no_wrap(line.character, body_font(), ink);
    paint_left(&painter, grid.character, middle, character, ink);
    let status_colour = status_ink(line.status, cursor);
    paint_left(
        &painter,
        grid.status,
        middle,
        roster_caps(&painter, line.status.text(), status_colour),
        status_colour,
    );
    let (ping, ping_ink) = match line.ping {
        PingCell::Local => (roster_caps(&painter, LOCAL_PING_TEXT, muted), muted),
        PingCell::Millis(millis) => (
            painter.layout_no_wrap(format!("{millis} ms"), body_font(), ink),
            ink,
        ),
        PingCell::Unknown => (
            painter.layout_no_wrap(NOTHING_YET_TEXT.to_owned(), body_font(), muted),
            muted,
        ),
    };
    paint_right(&painter, grid.ping.max, middle, ping, ping_ink);
    let Some((button, label)) = remove else {
        return false;
    };
    painter.rect_stroke(
        button,
        0.0,
        Stroke::new(1.0, INK_ON_SAND),
        eframe::egui::StrokeKind::Inside,
    );
    painter.galley(
        pos2(
            button.left() + REMOVE_PADDING_PX.0,
            button.center().y - label.size().y / 2.0,
        ),
        label,
        INK_ON_SAND,
    );
    ui.interact(
        button,
        ui.scope_id().with(("roster-remove", number)),
        Sense::click(),
    )
    .clicked()
}

fn remove_rect(painter: &Painter, grid: &RosterGrid, middle: f32) -> (Rect, Arc<Galley>) {
    let label = roster_caps(painter, REMOVE_LABEL, INK_ON_SAND);
    let size = label.size() + vec2(REMOVE_PADDING_PX.0, REMOVE_PADDING_PX.1) * 2.0;
    let button = Rect::from_min_size(
        pos2(
            grid.role.max - REMOVE_TO_CHARACTER_GAP_PX - size.x,
            middle - size.y / 2.0,
        ),
        size,
    );
    (button, label)
}

fn draw_name(
    ui: &mut Ui,
    cell: Rangef,
    middle: f32,
    name: SeatName,
    you: bool,
    (ink, muted): (Color32, Color32),
) {
    let painter = ui.painter().clone();
    let clip = Rect::from_x_y_ranges(
        cell,
        Rangef::new(
            middle - ROSTER_ROW_HEIGHT_PX / 2.0,
            middle + ROSTER_ROW_HEIGHT_PX / 2.0,
        ),
    );
    let clipped = painter.with_clip_rect(clip.intersect(painter.clip_rect()));
    let (text_end, underline) = match name {
        SeatName::Fixed(text) => (
            paint_name_text(&clipped, cell.min, middle, text, ink),
            false,
        ),
        SeatName::Open(text) => (
            paint_name_text(&clipped, cell.min, middle, text, INK_UNAVAILABLE),
            false,
        ),
        SeatName::Typed(text) => {
            let width = painter
                .layout_no_wrap(text.clone(), body_font(), ink)
                .size()
                .x;
            let field_end = (cell.min + width + NAME_UNDERLINE_TAIL_PX).min(cell.max);
            let field =
                Rect::from_min_max(pos2(cell.min, clip.top()), pos2(field_end, clip.bottom()));
            ui.put(
                field,
                TextEdit::singleline(text)
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .font(body_font())
                    .text_color(ink)
                    .desired_width(field.width()),
            );
            (field_end, true)
        }
    };
    if underline {
        clipped.hline(
            cell.min..=text_end,
            middle + NAME_UNDERLINE_BELOW_CENTRE_PX,
            Stroke::new(1.0, muted),
        );
    }
    if you {
        let tag = roster_caps(&painter, YOU_TAG, muted);
        let left = text_end + NAME_TO_TAG_GAP_PX;
        if left + tag.size().x <= cell.max {
            paint_left(&painter, Rangef::new(left, cell.max), middle, tag, muted);
        }
    }
}

fn paint_name_text(painter: &Painter, left: f32, middle: f32, text: &str, ink: Color32) -> f32 {
    painter
        .text(
            pos2(left, middle),
            Align2::LEFT_CENTER,
            text,
            body_font(),
            ink,
        )
        .right()
}

fn paint_row_lines(painter: &Painter, rect: Rect, last: bool) {
    let stroke = Stroke::new(1.0, HAIRLINE);
    painter.hline(rect.x_range(), rect.top() + 0.5, stroke);
    if last {
        painter.hline(rect.x_range(), rect.bottom() - 0.5, stroke);
    }
}

fn roster_caps(painter: &Painter, text: &str, color: Color32) -> Arc<Galley> {
    painter.layout_job(spaced_job(
        &text.to_uppercase(),
        caps_font(),
        color,
        ROSTER_CAPS_SPACING_EM,
    ))
}

fn paint_left(painter: &Painter, cell: Rangef, middle: f32, galley: Arc<Galley>, ink: Color32) {
    painter.galley(pos2(cell.min, middle - galley.size().y / 2.0), galley, ink);
}

fn paint_right(painter: &Painter, right: f32, middle: f32, galley: Arc<Galley>, ink: Color32) {
    painter.galley(
        pos2(right - galley.size().x, middle - galley.size().y / 2.0),
        galley,
        ink,
    );
}

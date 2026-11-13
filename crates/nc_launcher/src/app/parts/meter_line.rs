use eframe::egui::{Align2, Sense, Ui, UiBuilder, pos2, vec2};

use crate::app::palette::{INK, INK_MUTED, INK_UNAVAILABLE};
use crate::app::parts::{ControlLook, segmented_meter};
use crate::app::style::body_font;

const METER_LINE_HEIGHT_PX: f32 = 54.0;
const METER_LINE_LEFT_PX: f32 = 12.0;
const METER_LINE_LABEL_MIDDLE_PX: f32 = 16.0;
const METER_LINE_METER_TOP_PX: f32 = 32.0;
const METER_LINE_METER_HEIGHT_PX: f32 = 16.0;

/// A labelled amount: the label on one line, the segmented meter and the amount as a percentage under it.
/// Returns true when the amount changed.
pub fn meter_line(
    ui: &mut Ui,
    label: &str,
    percent: &mut u16,
    max_percent: u16,
    cells: u32,
    look: ControlLook,
) -> bool {
    let (rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), METER_LINE_HEIGHT_PX),
        Sense::hover(),
    );
    let ink = match look {
        ControlLook::Editable => INK,
        ControlLook::ReadOnly => INK_MUTED,
        ControlLook::Unavailable => INK_UNAVAILABLE,
    };
    let label_ink = if look == ControlLook::Unavailable {
        INK_UNAVAILABLE
    } else {
        INK
    };
    let left = rect.left() + METER_LINE_LEFT_PX;
    let painter = ui.painter();
    painter.text(
        pos2(left, rect.top() + METER_LINE_LABEL_MIDDLE_PX),
        Align2::LEFT_CENTER,
        label,
        body_font(),
        label_ink,
    );
    let meter_middle = rect.top() + METER_LINE_METER_TOP_PX + METER_LINE_METER_HEIGHT_PX / 2.0;
    painter.text(
        pos2(rect.right(), meter_middle),
        Align2::RIGHT_CENTER,
        format!("{percent} %"),
        body_font(),
        ink,
    );
    let meter_rect = rect
        .with_min_x(left)
        .with_min_y(rect.top() + METER_LINE_METER_TOP_PX)
        .with_max_y(rect.top() + METER_LINE_METER_TOP_PX + METER_LINE_METER_HEIGHT_PX);
    let mut value = u32::from(*percent);
    let changed = ui
        .scope_builder(UiBuilder::new().max_rect(meter_rect), |ui| {
            segmented_meter(ui, &mut value, u32::from(max_percent), cells, look).changed()
        })
        .inner;
    if changed {
        *percent = u16::try_from(value).unwrap_or(max_percent).min(max_percent);
    }
    changed
}

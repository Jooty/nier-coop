use eframe::egui::{
    Area, Color32, Id, Key, Order, Painter, Rect, Sense, Stroke, StrokeKind, Ui, vec2,
};

use crate::app::palette::{BAND, HAIRLINE_STRONG, HARD_SHADOW};

const DIALOG_SHADOW_OFFSET_PX: f32 = 6.0;
const DIM_ALPHA: u8 = 170;
const DIM_LAYER_SUFFIX: &str = "-dim";

/// A centred dialog over the dimmed window: `add` draws inside the dialog's rect. Returns true when the
/// player asked to close it, by Escape or a click on the dimmed window.
pub(super) fn show_dialog(
    ui: &mut Ui,
    size: (f32, f32),
    layer: &str,
    add: impl FnOnce(&mut Ui, Rect),
) -> bool {
    let whole = ui.ctx().content_rect();
    let mut close = ui.input(|input| input.key_pressed(Key::Escape));
    Area::new(Id::unique(format!("{layer}{DIM_LAYER_SUFFIX}")))
        .order(Order::Middle)
        .fixed_pos(whole.min)
        .show(ui.ctx(), |ui| {
            let (rect, response) = ui.allocate_exact_size(whole.size(), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(DIM_ALPHA));
            close |= response.clicked();
        });
    let dialog = Rect::from_center_size(whole.center(), vec2(size.0, size.1));
    Area::new(Id::unique(layer))
        .order(Order::Foreground)
        .fixed_pos(dialog.min)
        .show(ui.ctx(), |ui| {
            ui.set_min_size(dialog.size());
            paint_dialog_frame(ui.painter(), dialog);
            add(ui, dialog);
        });
    close
}

fn paint_dialog_frame(painter: &Painter, dialog: Rect) {
    painter.rect_filled(
        dialog.translate(vec2(DIALOG_SHADOW_OFFSET_PX, DIALOG_SHADOW_OFFSET_PX)),
        0.0,
        HARD_SHADOW,
    );
    painter.rect_filled(dialog, 0.0, BAND);
    painter.rect_stroke(
        dialog,
        0.0,
        Stroke::new(1.0, HAIRLINE_STRONG),
        StrokeKind::Inside,
    );
}

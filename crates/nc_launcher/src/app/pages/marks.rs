use eframe::egui::{Color32, Sense, Ui, Vec2};

const STATUS_DOT_DIAMETER_PX: f32 = 8.0;

pub(super) fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(STATUS_DOT_DIAMETER_PX), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), STATUS_DOT_DIAMETER_PX / 2.0, color);
}

use eframe::egui::{Color32, Painter, Rect, Stroke, pos2};

const HATCH_PERIOD_PX: f32 = 6.0;

/// Diagonal 1 px lines clipped to `rect`. Hatching always means "empty or not available".
pub fn paint_hatching(painter: &Painter, rect: Rect, color: Color32) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    // Lines run bottom-left to top-right, 6 px apart measured across them.
    let step = HATCH_PERIOD_PX * std::f32::consts::SQRT_2;
    let stroke = Stroke::new(1.0, color);
    let mut offset = 0.0;
    while offset < rect.width() + rect.height() {
        let start = pos2(rect.left() + offset - rect.height(), rect.bottom());
        let end = pos2(rect.left() + offset, rect.top());
        painter.line_segment([start, end], stroke);
        offset += step;
    }
}

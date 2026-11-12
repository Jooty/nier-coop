use eframe::egui::{Color32, Painter, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2, pos2, vec2};

use crate::app::{GameLook, GameMark};

const STATUS_DOT_DIAMETER_PX: f32 = 8.0;
const GAME_MARK_SIZE_PX: f32 = 10.0;
const HOUSE_ICON_SIZE_PX: f32 = 14.0;
const ICON_STROKE_WIDTH_PX: f32 = 1.4;

pub(super) fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(STATUS_DOT_DIAMETER_PX), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), STATUS_DOT_DIAMETER_PX / 2.0, color);
}

pub(super) fn game_mark(ui: &mut Ui, look: GameLook) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(GAME_MARK_SIZE_PX), Sense::hover());
    let center = rect.center();
    let half = GAME_MARK_SIZE_PX / 2.0;
    match look.mark {
        GameMark::Circle => {
            ui.painter().circle_filled(center, half, look.accent);
        }
        GameMark::Diamond => {
            // A 10 px square turned 45 degrees reaches half its diagonal from the centre.
            let reach = half * std::f32::consts::SQRT_2;
            ui.painter().add(Shape::convex_polygon(
                vec![
                    center + vec2(0.0, -reach),
                    center + vec2(reach, 0.0),
                    center + vec2(0.0, reach),
                    center + vec2(-reach, 0.0),
                ],
                look.accent,
                Stroke::NONE,
            ));
        }
    }
}

pub(super) fn house_icon_width() -> f32 {
    HOUSE_ICON_SIZE_PX
}

pub(super) fn paint_house_icon(painter: &Painter, left_center: Pos2, color: Color32) {
    let rect = Rect::from_min_size(
        pos2(left_center.x, left_center.y - HOUSE_ICON_SIZE_PX / 2.0),
        Vec2::splat(HOUSE_ICON_SIZE_PX),
    );
    let stroke = Stroke::new(ICON_STROKE_WIDTH_PX, color);
    let at = |x: f32, y: f32| {
        pos2(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        )
    };
    painter.add(Shape::closed_line(
        vec![
            at(0.5, 0.12),
            at(0.9, 0.46),
            at(0.8, 0.46),
            at(0.8, 0.88),
            at(0.2, 0.88),
            at(0.2, 0.46),
            at(0.1, 0.46),
        ],
        stroke,
    ));
    painter.line_segment([at(0.42, 0.88), at(0.42, 0.64)], stroke);
    painter.line_segment([at(0.42, 0.64), at(0.58, 0.64)], stroke);
    painter.line_segment([at(0.58, 0.64), at(0.58, 0.88)], stroke);
}

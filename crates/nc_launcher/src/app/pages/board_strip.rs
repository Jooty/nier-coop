use eframe::egui::epaint::RectShape;
use eframe::egui::{
    Color32, FontId, Mesh, Painter, Rect, Response, Sense, TextureHandle, Ui, pos2, vec2,
};
use nc_launcher_core::{StripArt, crop_share_around_focus};

use crate::app::pages::own_pick::OwnPick;
use crate::app::pages::own_pick::Step;
use crate::app::palette::{BAND, GROUND, INK, INK_MUTED};
use crate::app::parts::{caps_galley, paint_triangle_left, paint_triangle_right};
use crate::app::style::light_family;

pub(super) const BOARD_STRIP_HEIGHT_PX: f32 = 166.0;
const STRIP_DESIGN_WIDTH_PX: f32 = 820.0;
const STRIP_COVER_STOPS_PX: [(f32, f32); 5] = [
    (0.0, 1.0),
    (190.0, 1.0),
    (300.0, 0.84),
    (480.0, 0.42),
    (820.0, 0.16),
];
const YOUR_CHARACTER_KEY: &str = "Your character";
pub(super) const KEY_TOP_PX: f32 = 50.0;
pub(super) const KEY_TO_CHOOSER_PX: f32 = 8.0;
pub(super) const CHOOSER_SIZE_PX: f32 = 34.0;
pub(super) const CHOOSER_HEIGHT_PX: f32 = 40.0;
const CHOOSER_GAP_PX: f32 = 16.0;
const CHOOSER_ARROW_WIDTH_PX: f32 = 7.0;
const CHOOSER_HIT_SLOP_PX: f32 = 8.0;

/// The free roam board's head: the game's art as a strip fading leftward into the ground, and your
/// character's stepper over the faded end. Returns the step taken, if any.
pub(super) fn draw_board_strip(
    ui: &mut Ui,
    rect: Rect,
    art: Option<&TextureHandle>,
    crop: StripArt,
    own: &OwnPick,
) -> Option<Step> {
    match art {
        Some(art) => paint_strip_art(ui.painter(), rect, art, crop),
        None => {
            ui.painter().rect_filled(rect, 0.0, BAND);
        }
    }
    ui.painter().add(cover_mesh(rect));
    let key = caps_galley(ui.painter(), YOUR_CHARACTER_KEY, INK_MUTED);
    let key_top = rect.top() + KEY_TOP_PX;
    let chooser_top = key_top + key.size().y + KEY_TO_CHOOSER_PX;
    ui.painter()
        .galley(pos2(rect.left(), key_top), key, INK_MUTED);
    draw_chooser(ui, pos2(rect.left(), chooser_top), own)
}

fn paint_strip_art(painter: &Painter, rect: Rect, art: &TextureHandle, strip: StripArt) {
    let size = art.size_vec2();
    let crop = crop_share_around_focus(
        (size.x, size.y),
        (rect.width(), rect.height()),
        strip.focus,
        strip.height_share,
    );
    let uv = Rect::from_min_max(pos2(crop.min_u, crop.min_v), pos2(crop.max_u, crop.max_v));
    painter.add(RectShape::filled(rect, 0.0, Color32::WHITE).with_texture(art.id(), uv));
}

fn cover_mesh(rect: Rect) -> Mesh {
    let scale = rect.width() / STRIP_DESIGN_WIDTH_PX;
    let mut mesh = Mesh::default();
    for (index, (at_px, alpha)) in STRIP_COVER_STOPS_PX.into_iter().enumerate() {
        let x = rect.left() + at_px * scale;
        let color = GROUND.gamma_multiply(alpha);
        mesh.colored_vertex(pos2(x, rect.top()), color);
        mesh.colored_vertex(pos2(x, rect.bottom()), color);
        if index > 0 {
            let top_right = (index * 2) as u32;
            mesh.add_triangle(top_right - 2, top_right - 1, top_right);
            mesh.add_triangle(top_right - 1, top_right, top_right + 1);
        }
    }
    mesh
}

fn draw_chooser(ui: &mut Ui, top_left: eframe::egui::Pos2, own: &OwnPick) -> Option<Step> {
    let font = FontId::new(CHOOSER_SIZE_PX, light_family());
    let galley = ui.painter().layout_no_wrap(own.label.to_owned(), font, INK);
    let middle = top_left.y + CHOOSER_HEIGHT_PX / 2.0;
    if !own.steppable() {
        ui.painter().galley(
            pos2(top_left.x, middle - galley.size().y / 2.0),
            galley,
            INK,
        );
        return None;
    }
    let back_tip = pos2(top_left.x, middle);
    let text_left = back_tip.x + CHOOSER_ARROW_WIDTH_PX + CHOOSER_GAP_PX;
    let forward_left = text_left + galley.size().x + CHOOSER_GAP_PX;
    let forward_tip = pos2(forward_left + CHOOSER_ARROW_WIDTH_PX, middle);
    let back = arrow_response(ui, back_tip.x, middle, "board-step-back");
    let forward = arrow_response(ui, forward_left, middle, "board-step-forward");
    let painter = ui.painter();
    let arrow_ink = |response: &Response| if response.hovered() { INK } else { INK_MUTED };
    paint_triangle_left(painter, back_tip, arrow_ink(&back));
    paint_triangle_right(painter, forward_tip, arrow_ink(&forward));
    painter.galley(pos2(text_left, middle - galley.size().y / 2.0), galley, INK);
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
        pos2(left, middle - CHOOSER_HEIGHT_PX / 2.0),
        vec2(CHOOSER_ARROW_WIDTH_PX, CHOOSER_HEIGHT_PX),
    )
    .expand2(vec2(CHOOSER_HIT_SLOP_PX, 0.0));
    ui.interact(rect, ui.scope_id().with(salt), Sense::click())
}

use eframe::egui::epaint::RectShape;
use eframe::egui::{Color32, CornerRadius, Mesh, Painter, Rect, TextureHandle, pos2};
use nc_launcher_core::{ArtFocus, crop_around_focus, fade_stops};

use crate::app::palette::BAND;

const ART_TINT: Color32 = Color32::from_gray(190);
const FADE_VERTEX_COLUMNS: usize = 9;
const FADE_SHARE_OF_ART_WIDTH: f32 = 0.5;
const MIN_FADE_WIDTH_PX: f32 = 110.0;
const MAX_FADE_WIDTH_PX: f32 = 340.0;
const FOCUS_ANCHOR_Y: f32 = 0.5;

pub(super) fn paint_banner_art(
    painter: &Painter,
    banner: Rect,
    corner_radius: u8,
    fade_start_x: f32,
    art: &TextureHandle,
    focus: ArtFocus,
) {
    let art_rect = Rect::from_min_max(pos2(fade_start_x, banner.top()), banner.right_bottom());
    if art_rect.width() <= 0.0 {
        return;
    }
    let fade_width = (art_rect.width() * FADE_SHARE_OF_ART_WIDTH)
        .clamp(MIN_FADE_WIDTH_PX, MAX_FADE_WIDTH_PX)
        .min(art_rect.width());
    // The focus lands in the middle of the part the fade leaves clear.
    let clear_middle = fade_width / 2.0 + (art_rect.width() - fade_width / 2.0) / 2.0;
    let anchor = (clear_middle / art_rect.width(), FOCUS_ANCHOR_Y);
    let image_size = art.size_vec2();
    let crop = crop_around_focus(
        (image_size.x, image_size.y),
        (art_rect.width(), art_rect.height()),
        focus,
        anchor,
    );
    let uv = Rect::from_min_max(pos2(crop.min_u, crop.min_v), pos2(crop.max_u, crop.max_v));
    let corners = CornerRadius {
        nw: 0,
        sw: 0,
        ne: corner_radius,
        se: corner_radius,
    };
    painter.add(RectShape::filled(art_rect, corners, ART_TINT).with_texture(art.id(), uv));
    painter.add(fade_mesh(art_rect, fade_width));
}

fn fade_mesh(art_rect: Rect, fade_width: f32) -> Mesh {
    let mut mesh = Mesh::default();
    for (index, stop) in fade_stops(FADE_VERTEX_COLUMNS).iter().enumerate() {
        let x = art_rect.left() + stop.at * fade_width;
        let color = BAND.gamma_multiply(stop.alpha);
        mesh.colored_vertex(pos2(x, art_rect.top()), color);
        mesh.colored_vertex(pos2(x, art_rect.bottom()), color);
        if index > 0 {
            let top_right = (index * 2) as u32;
            mesh.add_triangle(top_right - 2, top_right - 1, top_right);
            mesh.add_triangle(top_right - 1, top_right, top_right + 1);
        }
    }
    mesh
}

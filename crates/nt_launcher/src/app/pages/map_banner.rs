use eframe::egui::epaint::RectShape;
use eframe::egui::{
    Align2, Color32, FontId, Painter, Rect, Stroke, StrokeKind, TextureHandle, pos2, vec2,
};
use nt_launcher_core::{ArtFocus, crop_around_focus};

use crate::app::palette::{BAND, GROUND, HAIRLINE_STRONG, HATCH_LINE, INK};
use crate::app::parts::paint_hatching;
use crate::app::style::garamond_family;

const NAME_INSET_PX: f32 = 14.0;
const NAME_BAND_MIN_HEIGHT_PX: f32 = 30.0;
const NAME_BAND_SHARE_OF_HEIGHT: f32 = 0.3;
const NAME_BAND_ALPHA: u8 = 200;
const CENTRED: (f32, f32) = (0.5, 0.5);

/// A map's banner: the picture with the map's name on a dark band at its foot, or a plain hatched panel with
/// the name when the launcher has no picture of it.
pub(super) fn paint_map_banner(
    painter: &Painter,
    rect: Rect,
    art: Option<&TextureHandle>,
    name: &str,
    name_px: f32,
) {
    let font = FontId::new(name_px, garamond_family());
    match art {
        Some(art) => {
            let size = art.size_vec2();
            let crop = crop_around_focus(
                (size.x, size.y),
                (rect.width(), rect.height()),
                ArtFocus {
                    x: CENTRED.0,
                    y: CENTRED.1,
                },
                CENTRED,
            );
            let uv = Rect::from_min_max(pos2(crop.min_u, crop.min_v), pos2(crop.max_u, crop.max_v));
            painter.add(RectShape::filled(rect, 0.0, Color32::WHITE).with_texture(art.id(), uv));
            let band = Rect::from_min_max(
                pos2(
                    rect.left(),
                    rect.bottom()
                        - (rect.height() * NAME_BAND_SHARE_OF_HEIGHT).max(NAME_BAND_MIN_HEIGHT_PX),
                ),
                rect.right_bottom(),
            );
            painter.rect_filled(band, 0.0, GROUND.gamma_multiply_u8(NAME_BAND_ALPHA));
            painter.text(
                pos2(band.left() + NAME_INSET_PX, band.center().y),
                Align2::LEFT_CENTER,
                name,
                font,
                INK,
            );
        }
        None => {
            painter.rect_filled(rect, 0.0, BAND);
            paint_hatching(painter, rect, HATCH_LINE);
            let galley = painter.layout_no_wrap(name.to_owned(), font, INK);
            let backing = Rect::from_center_size(rect.center(), galley.size())
                .expand2(vec2(NAME_INSET_PX, NAME_INSET_PX / 2.0));
            painter.rect_filled(backing, 0.0, BAND);
            painter.galley(
                backing.min + vec2(NAME_INSET_PX, NAME_INSET_PX / 2.0),
                galley,
                INK,
            );
        }
    }
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, HAIRLINE_STRONG),
        StrokeKind::Inside,
    );
}

use eframe::egui::{CursorIcon, FontId, Mesh, Rect, Sense, Stroke, Ui, pos2};

use crate::app::pages::board_strip::{
    CHOOSER_HEIGHT_PX, CHOOSER_SIZE_PX, KEY_TO_CHOOSER_PX, KEY_TOP_PX,
};
use crate::app::palette::{GROUND, INK, INK_MUTED};
use crate::app::parts::caps_galley;
use crate::app::style::light_family;

const MAP_KEY: &str = "Map";
const RIGHT_INSET_PX: f32 = 22.0;
const SHADE_WIDTH_PX: f32 = 320.0;
const SHADE_ALPHA: f32 = 0.82;
const UNDERLINE_BELOW_NAME_PX: f32 = 3.0;
const MAP_PICK_SALT: &str = "strip-map-pick";

/// The map at the free roam strip's right end: MAP over the map's name. For the host the name opens the map
/// choice; returns true when it was clicked.
pub(super) fn draw_strip_map(ui: &mut Ui, strip: Rect, name: &str, pickable: bool) -> bool {
    ui.painter().add(shade_mesh(strip));
    let right = strip.right() - RIGHT_INSET_PX;
    let key = caps_galley(ui.painter(), MAP_KEY, INK_MUTED);
    let key_top = strip.top() + KEY_TOP_PX;
    let name_top = key_top + key.size().y + KEY_TO_CHOOSER_PX;
    let font = FontId::new(CHOOSER_SIZE_PX, light_family());
    let name_galley = ui.painter().layout_no_wrap(name.to_owned(), font, INK);
    let name_rect = Rect::from_min_size(
        pos2(
            right - name_galley.size().x,
            name_top + (CHOOSER_HEIGHT_PX - name_galley.size().y) / 2.0,
        ),
        name_galley.size(),
    );
    ui.painter()
        .galley(pos2(right - key.size().x, key_top), key, INK_MUTED);
    ui.painter().galley(name_rect.min, name_galley, INK);
    if !pickable {
        return false;
    }
    let block = Rect::from_min_max(pos2(name_rect.left(), key_top), name_rect.right_bottom());
    let response = ui
        .interact(block, ui.scope_id().with(MAP_PICK_SALT), Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    let underline_ink = if response.hovered() { INK } else { INK_MUTED };
    ui.painter().hline(
        name_rect.x_range(),
        name_rect.bottom() + UNDERLINE_BELOW_NAME_PX,
        Stroke::new(1.0, underline_ink),
    );
    response.clicked()
}

fn shade_mesh(strip: Rect) -> Mesh {
    let left = (strip.right() - SHADE_WIDTH_PX).max(strip.left());
    let clear = GROUND.gamma_multiply(0.0);
    let shaded = GROUND.gamma_multiply(SHADE_ALPHA);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(pos2(left, strip.top()), clear);
    mesh.colored_vertex(pos2(left, strip.bottom()), clear);
    mesh.colored_vertex(strip.right_top(), shaded);
    mesh.colored_vertex(strip.right_bottom(), shaded);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    mesh
}

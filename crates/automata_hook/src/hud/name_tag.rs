use std::sync::atomic::{AtomicU8, Ordering};

use automata_game::address::CAMERA_OBJECT;
use automata_game::partner_hud::{
    CAMERA_PROJECTION_MATRIX_OFFSET, CAMERA_TRANSFORM_POINTER_OFFSET, CAMERA_VIEW_MATRIX_OFFSET,
    HUD_REFERENCE_SCREEN_HEIGHT, Projected, TAG_HEAD_OFFSET_UNITS, TagCamera, project_tag,
    tag_fade_alpha, tag_label,
};
use hudhook::imgui::{DrawListMut, Ui};
use nc_hook::diagnostics::log_line;

use crate::hud::assets;
use crate::hud::install::calls;
use crate::hud::partner_view::partner_view;
use crate::hud::settings::distance_shown;
use crate::hud::{
    ARROW_HALF_LENGTH_REFERENCE_PIXELS, ARROW_HALF_WIDTH_REFERENCE_PIXELS,
    ARROW_LABEL_GAP_REFERENCE_PIXELS, ARROW_OUTLINE_REFERENCE_PIXELS, TAG_LIFT_REFERENCE_PIXELS,
    TAG_TEXT_CELL_REFERENCE_PIXELS, TEXT_SHADOW_ALPHA, TEXT_SHADOW_OFFSET_REFERENCE_PIXELS,
};

const NOTHING_DRAWN_YET: u8 = 0;
const DRAWN_OVER_THE_BODY: u8 = 1;
const DRAWN_AS_AN_EDGE_ARROW: u8 = 2;
const CAMERA_NOT_READABLE: u8 = 3;

static FIRST_OF_EACH_KIND_LOGGED: AtomicU8 = AtomicU8::new(NOTHING_DRAWN_YET);

struct TagLook {
    scale: f32,
    alpha: f32,
}

pub(crate) fn draw_name_tag(ui: &Ui) {
    let Some(view) = partner_view() else {
        return;
    };
    let Some(camera) = read_camera() else {
        log_first(CAMERA_NOT_READABLE, || {
            "partner tag: the game camera's matrices are not readable, no tag is drawn".to_owned()
        });
        return;
    };
    let [width, height] = ui.io().display_size;
    let head = [
        view.position[0],
        view.position[1] + TAG_HEAD_OFFSET_UNITS,
        view.position[2],
    ];
    let look = TagLook {
        scale: height / HUD_REFERENCE_SCREEN_HEIGHT,
        alpha: tag_fade_alpha(view.distance),
    };
    let label = tag_label(&view.name, distance_shown().then_some(view.distance));
    match project_tag(&camera, head, width, height) {
        Projected::OnScreen { x, y } => {
            log_first(DRAWN_OVER_THE_BODY, || {
                format!(
                    "partner tag: first drawn over the body at {x:.0},{y:.0} of {width:.0}x{height:.0}, {:.1} units away",
                    view.distance
                )
            });
            draw_tag(ui, &label, [x, y], &look);
        }
        Projected::EdgeArrow {
            x,
            y,
            angle_radians,
        } => {
            log_first(DRAWN_AS_AN_EDGE_ARROW, || {
                format!(
                    "partner tag: first drawn as an edge arrow at {x:.0},{y:.0} of {width:.0}x{height:.0}, {:.1} units away",
                    view.distance
                )
            });
            draw_arrow(ui, &label, [x, y], angle_radians, [width, height], &look);
        }
        Projected::Invalid => {}
    }
}

pub(super) fn read_camera() -> Option<TagCamera> {
    let calls = calls()?;
    let transform = calls
        .read_u64_at(
            calls.address_value(CAMERA_OBJECT) + u64::from(CAMERA_TRANSFORM_POINTER_OFFSET),
        )
        .ok()
        .filter(|transform| *transform != 0)?;
    let camera = TagCamera {
        view: calls
            .read_f32s_at(transform + CAMERA_VIEW_MATRIX_OFFSET)
            .ok()?,
        projection: calls
            .read_f32s_at(transform + CAMERA_PROJECTION_MATRIX_OFFSET)
            .ok()?,
    };
    camera
        .view
        .iter()
        .chain(camera.projection.iter())
        .all(|value| value.is_finite())
        .then_some(camera)
}

fn draw_tag(ui: &Ui, label: &str, at: [f32; 2], look: &TagLook) {
    let cell = TAG_TEXT_CELL_REFERENCE_PIXELS * look.scale;
    let text = assets::measure(ui, label, cell);
    let label_at = [
        at[0] - text.width * 0.5,
        at[1] - TAG_LIFT_REFERENCE_PIXELS * look.scale - text.ink_bottom,
    ];
    draw_label(&ui.get_background_draw_list(), label, label_at, look);
}

fn draw_arrow(
    ui: &Ui,
    label: &str,
    at: [f32; 2],
    angle_radians: f32,
    screen: [f32; 2],
    look: &TagLook,
) {
    let (sin, cos) = angle_radians.sin_cos();
    // the angle is measured with up as positive; screen y points down
    let forward = [cos, -sin];
    let side = [sin, cos];
    let point = |along: f32, across: f32| {
        [
            at[0] + (forward[0] * along + side[0] * across) * look.scale,
            at[1] + (forward[1] * along + side[1] * across) * look.scale,
        ]
    };
    let palette = assets::palette();
    let draw_list = ui.get_background_draw_list();
    let (length, half_width, outline) = (
        ARROW_HALF_LENGTH_REFERENCE_PIXELS,
        ARROW_HALF_WIDTH_REFERENCE_PIXELS,
        ARROW_OUTLINE_REFERENCE_PIXELS,
    );
    draw_list
        .add_triangle(
            point(length + outline * 2.0, 0.0),
            point(-length - outline, half_width + outline * 1.5),
            point(-length - outline, -half_width - outline * 1.5),
            with_alpha(palette.dark, TEXT_SHADOW_ALPHA * look.alpha),
        )
        .filled(true)
        .build();
    draw_list
        .add_triangle(
            point(length, 0.0),
            point(-length, half_width),
            point(-length, -half_width),
            with_alpha(palette.light, look.alpha),
        )
        .filled(true)
        .build();
    let text = assets::measure(ui, label, TAG_TEXT_CELL_REFERENCE_PIXELS * look.scale);
    let gap = ARROW_LABEL_GAP_REFERENCE_PIXELS * look.scale;
    let ink_height = text.ink_bottom - text.ink_top;
    let label_at = [
        (at[0] - forward[0] * gap - text.width * 0.5).clamp(0.0, (screen[0] - text.width).max(0.0)),
        (at[1] - forward[1] * gap - ink_height * 0.5).clamp(0.0, (screen[1] - ink_height).max(0.0))
            - text.ink_top,
    ];
    draw_label(&draw_list, label, label_at, look);
}

fn draw_label(draw_list: &DrawListMut<'_>, label: &str, at: [f32; 2], look: &TagLook) {
    let palette = assets::palette();
    assets::draw_text(
        draw_list,
        label,
        at,
        TAG_TEXT_CELL_REFERENCE_PIXELS * look.scale,
        with_alpha(palette.light, look.alpha),
        (
            with_alpha(palette.dark, TEXT_SHADOW_ALPHA * look.alpha),
            TEXT_SHADOW_OFFSET_REFERENCE_PIXELS * look.scale,
        ),
    );
}

fn with_alpha(color: [f32; 3], alpha: f32) -> [f32; 4] {
    [color[0], color[1], color[2], alpha]
}

fn log_first(kind: u8, line: impl FnOnce() -> String) {
    let bit = 1_u8 << kind;
    if FIRST_OF_EACH_KIND_LOGGED.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
        log_line(&line());
    }
}

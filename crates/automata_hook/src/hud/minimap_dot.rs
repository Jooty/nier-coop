use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};

use automata_game::partner_hud::{
    HUD_REFERENCE_SCREEN_HEIGHT, HUD_REFERENCE_SCREEN_WIDTH, MINIMAP_CENTER_X_RANGE,
    MINIMAP_CENTER_Y_RANGE, MINIMAP_DOT_SIZE_RANGE, MINIMAP_RADIUS_RANGE,
    MINIMAP_WORLD_RANGE_RANGE, MinimapTuning, minimap_dot,
};
use hudhook::imgui::{TreeNodeFlags, Ui};
use nc_hook::diagnostics::log_line;

use crate::hud::assets;
use crate::hud::name_tag::read_camera;
use crate::hud::partner_view::partner_view;
use crate::hud::settings::minimap_dot_shown;
use crate::hud::{
    MINIMAP_DOT_OUTLINE_REFERENCE_PIXELS, MINIMAP_DOT_STATE_LINES_LOGGED_AT_MOST,
    MINIMAP_TUNING_SLIDER_WIDTH_PIXELS,
};

const STATE_UNKNOWN: u8 = 0;
const STATE_HIDDEN: u8 = 1;
const STATE_INSIDE: u8 = 2;
const STATE_ON_THE_RIM: u8 = 3;

static TUNING: Mutex<MinimapTuning> = Mutex::new(MinimapTuning::new());
static LAST_LOGGED_STATE: AtomicU8 = AtomicU8::new(STATE_UNKNOWN);
static STATE_LINES_LOGGED: AtomicU8 = AtomicU8::new(0);

pub(crate) fn draw_minimap_dot(ui: &Ui) {
    let tuning = *TUNING.lock().unwrap_or_else(PoisonError::into_inner);
    if !(minimap_dot_shown() && tuning.enabled) {
        return;
    }
    let dot = partner_view()
        .zip(read_camera())
        .and_then(|(view, camera)| {
            minimap_dot(view.local_position, view.position, &camera, &tuning)
        });
    let Some(dot) = dot else {
        log_state_change(STATE_HIDDEN, [0.0, 0.0]);
        return;
    };
    let [width, height] = ui.io().display_size;
    let scale = height / HUD_REFERENCE_SCREEN_HEIGHT;
    // The game's minimap keeps its distance from the right edge on every screen shape.
    let at = [
        width - (HUD_REFERENCE_SCREEN_WIDTH - tuning.center_x) * scale + dot.offset_x * scale,
        tuning.center_y * scale + dot.offset_y * scale,
    ];
    log_state_change(
        if dot.clamped_to_edge {
            STATE_ON_THE_RIM
        } else {
            STATE_INSIDE
        },
        [dot.offset_x, dot.offset_y],
    );
    let palette = assets::palette();
    let size = tuning.dot_size * scale;
    let draw_list = ui.get_background_draw_list();
    draw_list
        .add_circle(
            at,
            size + MINIMAP_DOT_OUTLINE_REFERENCE_PIXELS * scale,
            [palette.dark[0], palette.dark[1], palette.dark[2], 0.9],
        )
        .filled(true)
        .build();
    draw_list
        .add_circle(
            at,
            size,
            [palette.accent[0], palette.accent[1], palette.accent[2], 1.0],
        )
        .filled(true)
        .build();
}

pub(super) fn draw_minimap_tuning(ui: &Ui) {
    if !ui.collapsing_header("Minimap dot placement", TreeNodeFlags::empty()) {
        return;
    }
    let before = *TUNING.lock().unwrap_or_else(PoisonError::into_inner);
    let mut tuning = before;
    ui.checkbox(
        "The minimap turns with the camera",
        &mut tuning.rotate_with_camera,
    );
    slider(ui, "Centre X", &mut tuning.center_x, MINIMAP_CENTER_X_RANGE);
    slider(ui, "Centre Y", &mut tuning.center_y, MINIMAP_CENTER_Y_RANGE);
    slider(ui, "Radius", &mut tuning.radius, MINIMAP_RADIUS_RANGE);
    slider(
        ui,
        "World units to the rim",
        &mut tuning.world_range,
        MINIMAP_WORLD_RANGE_RANGE,
    );
    slider(ui, "Dot size", &mut tuning.dot_size, MINIMAP_DOT_SIZE_RANGE);
    if ui.button("Reset the minimap dot") {
        tuning = MinimapTuning::new();
    }
    ui.text_disabled("In pixels of a 1080-line screen, measured from its right edge.");
    if tuning != before {
        *TUNING.lock().unwrap_or_else(PoisonError::into_inner) = tuning;
        log_line(&format!(
            "minimap dot: placement now centre {:.0},{:.0}, radius {:.0}, {:.0} world units to the rim, dot {:.0}, turns with the camera: {}",
            tuning.center_x,
            tuning.center_y,
            tuning.radius,
            tuning.world_range,
            tuning.dot_size,
            tuning.rotate_with_camera
        ));
    }
}

fn slider(ui: &Ui, label: &str, value: &mut f32, range: (f32, f32)) {
    ui.set_next_item_width(MINIMAP_TUNING_SLIDER_WIDTH_PIXELS);
    ui.slider_config(label, range.0, range.1)
        .display_format("%.0f")
        .build(value);
}

fn log_state_change(state: u8, offset: [f32; 2]) {
    if LAST_LOGGED_STATE.swap(state, Ordering::Relaxed) == state
        || STATE_LINES_LOGGED.fetch_add(1, Ordering::Relaxed)
            >= MINIMAP_DOT_STATE_LINES_LOGGED_AT_MOST
    {
        return;
    }
    let name = match state {
        STATE_HIDDEN => "hidden",
        STATE_INSIDE => "inside the rim",
        _ => "on the rim",
    };
    log_line(&format!(
        "minimap dot: {name} (offset {:.0},{:.0} of a 1080-line screen; the placement is a guess until it is set by eye)",
        offset[0], offset[1]
    ));
}

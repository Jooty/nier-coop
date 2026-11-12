use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use automata_game::address::CAMERA_OBJECT;
use automata_game::partner_hud::{
    CAMERA_TRANSFORM_POINTER_OFFSET, CAMERA_VIEWPORT_OFFSET, HUD_REFERENCE_SCREEN_HEIGHT,
    MINIMAP_DOT_SIZE_RANGE, MINIMAP_NUDGE_RANGE, MinimapDot, MinimapTuning, minimap_dot,
    minimap_hidden_reason, ui_point_on_display,
};
use hudhook::imgui::{TreeNodeFlags, Ui};
use nc_hook::diagnostics::log_line;

use crate::hud::assets;
use crate::hud::install::calls;
use crate::hud::minimap_hook::{MinimapReading, last_minimap_reading};
use crate::hud::partner_view::{PartnerView, partner_views};
use crate::hud::settings::minimap_dot_shown;
use crate::hud::{
    MINIMAP_DOT_LOG_INTERVAL_MS, MINIMAP_DOT_OUTLINE_REFERENCE_PIXELS,
    MINIMAP_DOT_PLACEMENT_LINES_LOGGED_AT_MOST, MINIMAP_DOT_STATE_LINES_LOGGED_AT_MOST,
    MINIMAP_TUNING_SLIDER_WIDTH_PIXELS,
};

struct HiddenLog {
    last: Option<Option<&'static str>>,
    lines: u8,
}

static TUNING: Mutex<MinimapTuning> = Mutex::new(MinimapTuning::new());
static HIDDEN_LOG: Mutex<HiddenLog> = Mutex::new(HiddenLog {
    last: None,
    lines: 0,
});
static LAST_PLACEMENT_LOG: Mutex<Option<Instant>> = Mutex::new(None);
static PLACEMENT_LINES_LOGGED: AtomicU32 = AtomicU32::new(0);

pub(crate) fn draw_minimap_dot(ui: &Ui) {
    if !minimap_dot_shown() {
        return;
    }
    let views = partner_views();
    if views.is_empty() {
        return;
    }
    let reading = last_minimap_reading();
    let age_ms = reading.map_or(0, |reading| reading.read_at.elapsed().as_millis() as u64);
    let hidden = minimap_hidden_reason(reading.as_ref().map(|reading| &reading.map), age_ms);
    log_hidden_change(hidden);
    let Some(reading) = reading.filter(|_| hidden.is_none()) else {
        return;
    };
    let Some(viewport) = read_viewport() else {
        log_hidden_change(Some("the camera viewport is not readable"));
        return;
    };
    for view in &views {
        draw_one_minimap_dot(ui, view, &reading, viewport);
    }
}

fn draw_one_minimap_dot(ui: &Ui, view: &PartnerView, reading: &MinimapReading, viewport: [i32; 4]) {
    let Some(dot) = minimap_dot(view.local_position, view.position, &reading.map) else {
        return;
    };
    let display = ui.io().display_size;
    let Some(on_display) = ui_point_on_display([dot.x, dot.y], viewport, display) else {
        return;
    };
    let tuning = *TUNING.lock().unwrap_or_else(PoisonError::into_inner);
    let scale = display[1] / HUD_REFERENCE_SCREEN_HEIGHT;
    let at = [
        on_display[0] + tuning.nudge_x * scale,
        on_display[1] + tuning.nudge_y * scale,
    ];
    log_placement(reading, viewport, &dot, at);
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
    slider(ui, "Nudge right", &mut tuning.nudge_x, MINIMAP_NUDGE_RANGE);
    slider(ui, "Nudge down", &mut tuning.nudge_y, MINIMAP_NUDGE_RANGE);
    slider(ui, "Dot size", &mut tuning.dot_size, MINIMAP_DOT_SIZE_RANGE);
    if ui.button("Reset the minimap dot") {
        tuning = MinimapTuning::new();
    }
    ui.text_disabled(
        "The game's minimap places the dot; these nudge it, in pixels of a 1080-line screen.",
    );
    if tuning != before {
        *TUNING.lock().unwrap_or_else(PoisonError::into_inner) = tuning;
        log_line(&format!(
            "minimap dot: nudge now {:.0},{:.0}, dot {:.0}",
            tuning.nudge_x, tuning.nudge_y, tuning.dot_size
        ));
    }
}

fn read_viewport() -> Option<[i32; 4]> {
    let calls = calls()?;
    let transform = calls
        .read_u64_at(
            calls.address_value(CAMERA_OBJECT) + u64::from(CAMERA_TRANSFORM_POINTER_OFFSET),
        )
        .ok()
        .filter(|transform| *transform != 0)?;
    let viewport = calls
        .read_u32s_at::<4>(transform + CAMERA_VIEWPORT_OFFSET)
        .ok()?;
    Some(viewport.map(|value| value as i32))
}

fn slider(ui: &Ui, label: &str, value: &mut f32, range: (f32, f32)) {
    ui.set_next_item_width(MINIMAP_TUNING_SLIDER_WIDTH_PIXELS);
    ui.slider_config(label, range.0, range.1)
        .display_format("%.0f")
        .build(value);
}

fn log_hidden_change(hidden: Option<&'static str>) {
    let mut log = HIDDEN_LOG.lock().unwrap_or_else(PoisonError::into_inner);
    if log.last == Some(hidden) || log.lines >= MINIMAP_DOT_STATE_LINES_LOGGED_AT_MOST {
        return;
    }
    log.last = Some(hidden);
    log.lines += 1;
    match hidden {
        Some(reason) => log_line(&format!("minimap dot: hidden ({reason})")),
        None => log_line("minimap dot: shown over the game's minimap"),
    }
}

fn log_placement(reading: &MinimapReading, viewport: [i32; 4], dot: &MinimapDot, at: [f32; 2]) {
    let mut last = LAST_PLACEMENT_LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if last.is_some_and(|logged| {
        logged.elapsed().as_millis() < u128::from(MINIMAP_DOT_LOG_INTERVAL_MS)
    }) {
        return;
    }
    *last = Some(Instant::now());
    if PLACEMENT_LINES_LOGGED.fetch_add(1, Ordering::Relaxed)
        >= MINIMAP_DOT_PLACEMENT_LINES_LOGGED_AT_MOST
    {
        return;
    }
    let map = &reading.map;
    log_line(&format!(
        "minimap dot: game map state {} kind {}, centre {:.1},{:.1}, rect x {:.1}..{:.1} y {:.1}..{:.1}, {:.4} ui units per world unit (zoom {:.3}), rotation {:.3} rad, viewport {},{} {}x{}; dot at ui {:.1},{:.1} -> screen {:.0},{:.0}, on the rim: {}",
        map.open_state,
        map.kind,
        map.centre[0],
        map.centre[1],
        map.left,
        map.right,
        map.bottom,
        map.top,
        map.ui_units_per_world_unit,
        reading.zoom,
        map.rotation_radians,
        viewport[0],
        viewport[1],
        viewport[2],
        viewport[3],
        dot.x,
        dot.y,
        at[0],
        at[1],
        dot.clamped_to_edge
    ));
}

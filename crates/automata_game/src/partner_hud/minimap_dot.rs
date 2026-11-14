use crate::exe::ExeVariant;
use crate::partner_hud::{
    MINIMAP_DOT_EDGE_INSET_FRACTION, MINIMAP_OPEN_STATE, MINIMAP_OPENING_STATE,
    MINIMAP_READING_STALE_MS, MINIMAP_SIDE_VIEW_KIND, MINIMAP_UPDATE_FUNCTION_COMPAT_START,
    MINIMAP_UPDATE_FUNCTION_MAIN_START,
};

/// The game's own minimap as its last update left it. Positions are in the game's UI units,
/// which the game compares with camera viewport pixels (x right, y down).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameMinimap {
    pub open_state: i32,
    pub kind: i32,
    pub centre: [f32; 2],
    pub left: f32,
    pub top: f32,
    pub bottom: f32,
    pub right: f32,
    pub ui_units_per_world_unit: f32,
    pub rotation_radians: f32,
}

/// Where the partner sits on the game's minimap, in the game's UI units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapDot {
    pub x: f32,
    pub y: f32,
    pub clamped_to_edge: bool,
}

/// The fine adjustment on top of the game's own placement, in pixels of a 1080-line screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapTuning {
    pub nudge_x: f32,
    pub nudge_y: f32,
    pub dot_size: f32,
}

impl MinimapTuning {
    pub const fn new() -> Self {
        Self {
            nudge_x: 0.0,
            nudge_y: 0.0,
            dot_size: 7.0,
        }
    }
}

impl Default for MinimapTuning {
    fn default() -> Self {
        Self::new()
    }
}

pub fn minimap_update_start(variant: ExeVariant) -> &'static [u8] {
    match variant {
        ExeVariant::Main => &MINIMAP_UPDATE_FUNCTION_MAIN_START,
        ExeVariant::Compat => &MINIMAP_UPDATE_FUNCTION_COMPAT_START,
    }
}

/// Why the partner's dot is not drawn over the game's minimap, or `None` when it is.
pub fn minimap_hidden_reason(
    map: Option<&GameMinimap>,
    reading_age_ms: u64,
) -> Option<&'static str> {
    let Some(map) = map else {
        return Some("the game's minimap has not been updated yet");
    };
    if reading_age_ms > MINIMAP_READING_STALE_MS {
        return Some("the game no longer updates its minimap");
    }
    if map.open_state != MINIMAP_OPENING_STATE && map.open_state != MINIMAP_OPEN_STATE {
        return Some("the game's minimap is closed");
    }
    let sized = [
        map.centre[0],
        map.centre[1],
        map.left,
        map.top,
        map.bottom,
        map.right,
        map.rotation_radians,
        map.ui_units_per_world_unit,
    ]
    .iter()
    .all(|value| value.is_finite())
        && map.right > map.left
        && map.top > map.bottom
        && map.ui_units_per_world_unit > 0.0;
    (!sized).then_some("the game's minimap has no size")
}

/// The partner on the game's minimap, placed the way the game places its own icons (depth down
/// the map, or height in a side view), kept inside the circle the map's rectangle holds.
pub fn minimap_dot(local: [f32; 3], partner: [f32; 3], map: &GameMinimap) -> Option<MinimapDot> {
    if minimap_hidden_reason(Some(map), 0).is_some()
        || !local
            .iter()
            .chain(partner.iter())
            .all(|axis| axis.is_finite())
    {
        return None;
    }
    let across = partner[0] - local[0];
    let along = if map.kind == MINIMAP_SIDE_VIEW_KIND {
        local[1] - partner[1]
    } else {
        partner[2] - local[2]
    };
    let (sin, cos) = map.rotation_radians.sin_cos();
    let x = (across * cos - along * sin) * map.ui_units_per_world_unit;
    let y = (along * cos + across * sin) * map.ui_units_per_world_unit;
    let limit = (map.right - map.left).min(map.top - map.bottom)
        * 0.5
        * (1.0 - MINIMAP_DOT_EDGE_INSET_FRACTION);
    let length = x.hypot(y);
    let (x, y, clamped_to_edge) = if length <= limit {
        (x, y, false)
    } else {
        (x / length * limit, y / length * limit, true)
    };
    Some(MinimapDot {
        x: map.centre[0] + x,
        y: map.centre[1] + y,
        clamped_to_edge,
    })
}

/// A point in the game's UI units on the display, through the camera viewport
/// (`[x, y, width, height]` in pixels).
pub fn ui_point_on_display(
    point: [f32; 2],
    viewport: [i32; 4],
    display: [f32; 2],
) -> Option<[f32; 2]> {
    let [x, y, width, height] = viewport.map(|value| value as f32);
    let usable = width > 0.0
        && height > 0.0
        && display.iter().all(|side| side.is_finite() && *side > 0.0)
        && point.iter().all(|axis| axis.is_finite());
    usable.then(|| {
        [
            (point[0] - x) / width * display[0],
            (point[1] - y) / height * display[1],
        ]
    })
}

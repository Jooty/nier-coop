use crate::partner_hud::{MINIMAP_DOT_EDGE_INSET_FRACTION, TagCamera};

/// Where the game's minimap is and how much world it shows, in pixels of a 1080-line screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapTuning {
    pub enabled: bool,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub world_range: f32,
    pub dot_size: f32,
    pub rotate_with_camera: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapDot {
    pub offset_x: f32,
    pub offset_y: f32,
    pub clamped_to_edge: bool,
}

impl MinimapTuning {
    pub const fn new() -> Self {
        Self {
            enabled: true,
            center_x: 1742.0,
            center_y: 178.0,
            radius: 118.0,
            world_range: 110.0,
            dot_size: 7.0,
            rotate_with_camera: true,
        }
    }
}

impl Default for MinimapTuning {
    fn default() -> Self {
        Self::new()
    }
}

/// The partner's offset from the minimap's centre, in the tuning's pixels: up is the camera's
/// forward when the map turns with the camera, else minus world z; kept inside the rim.
pub fn minimap_dot(
    local: [f32; 3],
    partner: [f32; 3],
    camera: &TagCamera,
    tuning: &MinimapTuning,
) -> Option<MinimapDot> {
    let usable = local
        .iter()
        .chain(partner.iter())
        .all(|axis| axis.is_finite())
        && tuning.world_range > 0.0
        && tuning.radius > 0.0;
    if !usable {
        return None;
    }
    let delta = [partner[0] - local[0], partner[2] - local[2]];
    let (right, up) = if tuning.rotate_with_camera {
        let (right, forward) = camera.ground_axes()?;
        (
            delta[0] * right[0] + delta[1] * right[1],
            delta[0] * forward[0] + delta[1] * forward[1],
        )
    } else {
        (delta[0], -delta[1])
    };
    let pixels_per_unit = tuning.radius / tuning.world_range;
    let (x, y) = (right * pixels_per_unit, -up * pixels_per_unit);
    let length = x.hypot(y);
    let limit = tuning.radius * (1.0 - MINIMAP_DOT_EDGE_INSET_FRACTION);
    if length <= limit {
        return Some(MinimapDot {
            offset_x: x,
            offset_y: y,
            clamped_to_edge: false,
        });
    }
    Some(MinimapDot {
        offset_x: x / length * limit,
        offset_y: y / length * limit,
        clamped_to_edge: true,
    })
}

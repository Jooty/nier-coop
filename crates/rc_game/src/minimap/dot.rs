use crate::marker::CameraState;
use crate::minimap::{MINIMAP_DOT_EDGE_INSET_FRACTION, MinimapTuning};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapDot {
    pub offset_x: f32,
    pub offset_y: f32,
    pub clamped_to_edge: bool,
}

pub fn minimap_dot(
    local: [f32; 3],
    partner: [f32; 3],
    camera: &CameraState,
    tuning: &MinimapTuning,
) -> Option<MinimapDot> {
    if !(local
        .iter()
        .chain(partner.iter())
        .all(|axis| axis.is_finite())
        && tuning.world_range > 0.0
        && tuning.radius > 0.0)
    {
        return None;
    }
    let delta = (partner[0] - local[0], partner[2] - local[2]);
    let (right, up) = if tuning.rotate_with_camera {
        camera_relative(camera, delta)?
    } else {
        (delta.0, delta.1)
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

fn camera_relative(camera: &CameraState, delta: (f32, f32)) -> Option<(f32, f32)> {
    let m = &camera.world_matrix;
    // same axes as marker::project: forward is column 2, right is minus column 0
    let forward = normalized((m[2], m[10]))?;
    let right = normalized((-m[0], -m[8]))?;
    Some((
        delta.0 * right.0 + delta.1 * right.1,
        delta.0 * forward.0 + delta.1 * forward.1,
    ))
}

fn normalized(vector: (f32, f32)) -> Option<(f32, f32)> {
    let length = vector.0.hypot(vector.1);
    (length.is_finite() && length > 1.0e-6).then(|| (vector.0 / length, vector.1 / length))
}

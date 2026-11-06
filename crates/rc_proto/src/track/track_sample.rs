use crate::wire::{MotionState, PlayerState};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSample {
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
}

impl TrackSample {
    pub fn of(state: &PlayerState) -> Self {
        Self {
            phase: state.phase,
            position: state.position,
            yaw_radians: state.yaw_radians,
            motion: state.motion,
        }
    }

    pub fn between(earlier: &PlayerState, later: &PlayerState, fraction: f32) -> Self {
        if earlier.phase != later.phase {
            return Self::of(earlier);
        }
        let fraction = fraction.clamp(0.0, 1.0);
        let position = std::array::from_fn(|axis| {
            earlier.position[axis] + (later.position[axis] - earlier.position[axis]) * fraction
        });
        Self {
            position,
            yaw_radians: lerp_angle(earlier.yaw_radians, later.yaw_radians, fraction),
            ..Self::of(earlier)
        }
    }
}

fn lerp_angle(from: f32, to: f32, fraction: f32) -> f32 {
    let turn =
        (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + turn * fraction
}

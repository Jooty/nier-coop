use crate::track::{MAX_MOTION_EXTRAPOLATION_MS, MOTION_FRAMES_PER_MS};
use crate::wire::{MotionState, PartnerActivity, PlayerState, RiddenBoar, WeaponState};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSample {
    pub phase: u16,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
    pub weapon: Option<WeaponState>,
    pub activity: PartnerActivity,
    pub ridden_boar: Option<RiddenBoar>,
}

impl TrackSample {
    pub fn of(state: &PlayerState) -> Self {
        Self {
            phase: state.phase,
            position: state.position,
            yaw_radians: state.yaw_radians,
            motion: state.motion,
            weapon: state.weapon,
            activity: state.activity,
            ridden_boar: state.ridden_boar,
        }
    }

    /// The newest state, `elapsed_ms` after it arrived: the motion keeps playing at its speed
    /// for a short while, so a late state does not freeze the puppet's frame.
    pub fn held(state: &PlayerState, elapsed_ms: u64) -> Self {
        Self {
            motion: state.motion.map(|motion| advanced(motion, elapsed_ms)),
            ridden_boar: state.ridden_boar.map(|boar| RiddenBoar {
                motion: boar.motion.map(|motion| advanced(motion, elapsed_ms)),
                ..boar
            }),
            ..Self::of(state)
        }
    }

    pub fn between(
        earlier: &PlayerState,
        later: &PlayerState,
        fraction: f32,
        elapsed_ms: u64,
    ) -> Self {
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
            motion: earlier
                .motion
                .map(|motion| motion_between(motion, later.motion, fraction, elapsed_ms)),
            ridden_boar: earlier
                .ridden_boar
                .map(|boar| boar_between(boar, later.ridden_boar, fraction, elapsed_ms)),
            ..Self::of(earlier)
        }
    }
}

fn boar_between(
    earlier: RiddenBoar,
    later: Option<RiddenBoar>,
    fraction: f32,
    elapsed_ms: u64,
) -> RiddenBoar {
    let Some(later) = later else {
        return RiddenBoar {
            motion: earlier.motion.map(|motion| advanced(motion, elapsed_ms)),
            ..earlier
        };
    };
    RiddenBoar {
        offset_from_rider: std::array::from_fn(|axis| {
            earlier.offset_from_rider[axis]
                + (later.offset_from_rider[axis] - earlier.offset_from_rider[axis]) * fraction
        }),
        yaw_from_rider_radians: lerp_angle(
            earlier.yaw_from_rider_radians,
            later.yaw_from_rider_radians,
            fraction,
        ),
        motion: earlier
            .motion
            .map(|motion| motion_between(motion, later.motion, fraction, elapsed_ms)),
    }
}

fn motion_between(
    earlier: MotionState,
    later: Option<MotionState>,
    fraction: f32,
    elapsed_ms: u64,
) -> MotionState {
    match later {
        Some(later) if later.index == earlier.index && later.frame >= earlier.frame => {
            MotionState {
                frame: earlier.frame + (later.frame - earlier.frame) * fraction,
                ..earlier
            }
        }
        // A wrapped loop or the last frames before a new motion: play on at the earlier speed;
        // the puppet wraps or clamps by the motion's own frame count.
        _ => advanced(earlier, elapsed_ms),
    }
}

fn advanced(motion: MotionState, elapsed_ms: u64) -> MotionState {
    let elapsed = elapsed_ms.min(MAX_MOTION_EXTRAPOLATION_MS) as f32;
    MotionState {
        frame: motion.frame + motion.speed * elapsed * MOTION_FRAMES_PER_MS,
        ..motion
    }
}

fn lerp_angle(from: f32, to: f32, fraction: f32) -> f32 {
    let turn =
        (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + turn * fraction
}

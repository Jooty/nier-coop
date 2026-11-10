use core::f32::consts::{PI, TAU};

use crate::partner::{
    FACING_ERROR_WRITTEN_RADIANS, POSITION_ERROR_LEFT_ALONE, POSITION_ERROR_TELEPORTED,
    POSITION_NUDGE_FRACTION, TELEPORT_MIN_INTERVAL_MS,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionCorrection {
    Leave,
    Nudge([f32; 3]),
    Teleport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PositionCorrector {
    last_teleport_ms: Option<u64>,
}

impl PositionCorrector {
    pub fn correct(
        &mut self,
        now_ms: u64,
        local: [f32; 3],
        remote: [f32; 3],
    ) -> PositionCorrection {
        let error = [
            remote[0] - local[0],
            remote[1] - local[1],
            remote[2] - local[2],
        ];
        let distance = (error[0] * error[0] + error[1] * error[1] + error[2] * error[2]).sqrt();
        if !distance.is_finite() || distance < POSITION_ERROR_LEFT_ALONE {
            return PositionCorrection::Leave;
        }
        let teleport_allowed = self
            .last_teleport_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= TELEPORT_MIN_INTERVAL_MS);
        if distance > POSITION_ERROR_TELEPORTED && teleport_allowed {
            self.last_teleport_ms = Some(now_ms);
            return PositionCorrection::Teleport;
        }
        PositionCorrection::Nudge([
            local[0] + error[0] * POSITION_NUDGE_FRACTION,
            local[1] + error[1] * POSITION_NUDGE_FRACTION,
            local[2] + error[2] * POSITION_NUDGE_FRACTION,
        ])
    }
}

/// The shortest signed turn from `local` to `remote`, in radians.
pub fn facing_error(local: f32, remote: f32) -> f32 {
    let turn = (remote - local).rem_euclid(TAU);
    if turn > PI { turn - TAU } else { turn }
}

pub fn facing_needs_write(local: f32, remote: f32) -> bool {
    let error = facing_error(local, remote);
    error.is_finite() && error.abs() > FACING_ERROR_WRITTEN_RADIANS
}

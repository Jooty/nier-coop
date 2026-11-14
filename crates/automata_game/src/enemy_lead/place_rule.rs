use core::f32::consts::{PI, TAU};

use crate::enemy_lead::{
    FACING_ERROR_LEFT_ALONE_RADIANS, FACING_NUDGE_FRACTION_PER_FRAME, PLACE_ERROR_LEFT_ALONE,
    PLACE_ERROR_SNAPPED, PLACE_NUDGE_FRACTION_PER_FRAME,
};
use crate::partner::facing_error;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaceStep {
    Leave,
    MoveTo([f32; 3]),
}

/// Where this frame puts a guest's enemy that follows the host's place: near enough is left
/// alone, far is put on the host's place at once, between is eased a fraction of the way.
pub fn place_step(local: [f32; 3], host: [f32; 3]) -> PlaceStep {
    let error = [host[0] - local[0], host[1] - local[1], host[2] - local[2]];
    let distance = (error[0] * error[0] + error[1] * error[1] + error[2] * error[2]).sqrt();
    if !distance.is_finite() || distance < PLACE_ERROR_LEFT_ALONE {
        return PlaceStep::Leave;
    }
    if distance > PLACE_ERROR_SNAPPED {
        return PlaceStep::MoveTo(host);
    }
    PlaceStep::MoveTo([
        local[0] + error[0] * PLACE_NUDGE_FRACTION_PER_FRAME,
        local[1] + error[1] * PLACE_NUDGE_FRACTION_PER_FRAME,
        local[2] + error[2] * PLACE_NUDGE_FRACTION_PER_FRAME,
    ])
}

/// The facing to write this frame, eased along the shorter turn; none when it is close enough.
pub fn facing_to_write(local: f32, host: f32) -> Option<f32> {
    let turn = facing_error(local, host);
    if !turn.is_finite() || turn.abs() < FACING_ERROR_LEFT_ALONE_RADIANS {
        return None;
    }
    let eased = local + turn * FACING_NUDGE_FRACTION_PER_FRAME;
    Some((eased + PI).rem_euclid(TAU) - PI)
}

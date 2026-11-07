use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, ACTOR_PENDING_DISPLACEMENT_OFFSET};

const FLOAT_LANE_BYTES: u64 = 4;

/// What is left of a step a guest copy queued for itself: nothing for a held copy, whose place
/// is the host's, and only a fall for a dead one, so its body drops to the floor but never slides.
pub fn kept_step(step: [f32; 3], may_fall: bool) -> [f32; 3] {
    let fall = if may_fall && step[1].is_finite() {
        step[1].min(0.0)
    } else {
        0.0
    };
    [0.0, fall, 0.0]
}

/// Whether the guest holds its copy (no AI or state steps, the host's motion played on it). A
/// copy whose host body motion cannot be read would stand frozen, so it runs its own motions.
pub fn holds_copy(host_motion_known: bool) -> bool {
    host_motion_known
}

/// Cuts the step a copy's own motion, hit reaction or death queued at `actor + 0x190` for the
/// physics pass down to `kept_step`. Returns the horizontal length cut when a step was pending.
pub fn stop_own_step<M: GameMemory>(
    memory: &mut M,
    actor: u64,
    may_fall: bool,
) -> Result<Option<f32>, GameError> {
    let flag_address = actor + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET;
    if memory.read_u8(flag_address)? == 0 {
        return Ok(None);
    }
    let base = actor + ACTOR_PENDING_DISPLACEMENT_OFFSET;
    let step = [
        memory.read_f32(base)?,
        memory.read_f32(base + FLOAT_LANE_BYTES)?,
        memory.read_f32(base + 2 * FLOAT_LANE_BYTES)?,
    ];
    let kept = kept_step(step, may_fall);
    for (lane, value) in (0_u64..).zip(kept) {
        memory.write_f32(base + lane * FLOAT_LANE_BYTES, value)?;
    }
    if kept[1] >= 0.0 {
        memory.write_u8(flag_address, 0)?;
    }
    let cut = step[0].hypot(step[2]);
    Ok(Some(if cut.is_finite() { cut } else { f32::MAX }))
}

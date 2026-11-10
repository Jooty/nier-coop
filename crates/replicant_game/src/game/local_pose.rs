use crate::game::{GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{
    MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET, MOTION_LAYER_MAX_SLOT_COUNT,
    MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET, MOTION_LAYER_SLOT_COUNT_OFFSET,
    MOTION_PLAYER_BODY_LAYER_POINTER_OFFSET, MOTION_SLOT_FRAME_OFFSET,
    MOTION_SLOT_MOTION_INDEX_OFFSET, MOTION_SLOT_STRIDE, MOTION_WORK_MOTION_PLAYER_POINTER_OFFSET,
    PLAYABLE_ACTOR_FACING_X_OFFSET, PLAYABLE_ACTOR_FACING_Z_OFFSET,
    PLAYABLE_ACTOR_MOTION_WORK_OFFSET,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyMotion {
    pub index: i32,
    pub frame: f32,
}

impl<M: GameMemory> Replicant<M> {
    pub fn yaw_radians(&self) -> Result<f32, GameError> {
        let actor = self.playable_actor()?;
        let x = self
            .memory()
            .read_f32(actor + PLAYABLE_ACTOR_FACING_X_OFFSET)?;
        let z = self
            .memory()
            .read_f32(actor + PLAYABLE_ACTOR_FACING_Z_OFFSET)?;
        Ok(x.atan2(z))
    }

    pub fn body_motion(&self) -> Result<Option<BodyMotion>, GameError> {
        let memory = self.memory();
        let actor = self.playable_actor()?;
        let motion_work = actor + PLAYABLE_ACTOR_MOTION_WORK_OFFSET;
        let player = memory.read_u64(motion_work + MOTION_WORK_MOTION_PLAYER_POINTER_OFFSET)?;
        if player == 0 {
            return Ok(None);
        }
        let layer = memory.read_u64(player + MOTION_PLAYER_BODY_LAYER_POINTER_OFFSET)?;
        if layer == 0 {
            return Ok(None);
        }
        let current = memory.read_i32(layer + MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET)?;
        let count = memory.read_i32(layer + MOTION_LAYER_SLOT_COUNT_OFFSET)?;
        let slots = memory.read_u64(layer + MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET)?;
        let slot_in_range = (0..count.min(MOTION_LAYER_MAX_SLOT_COUNT)).contains(&current);
        if slots == 0 || !slot_in_range {
            return Ok(None);
        }
        let slot = slots + current as u64 * MOTION_SLOT_STRIDE;
        let index = memory.read_i32(slot + MOTION_SLOT_MOTION_INDEX_OFFSET)?;
        if index < 0 {
            return Ok(None);
        }
        let frame = memory.read_f32(slot + MOTION_SLOT_FRAME_OFFSET)?;
        Ok(Some(BodyMotion { index, frame }))
    }
}

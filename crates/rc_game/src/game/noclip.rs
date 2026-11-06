use crate::game::{Displacement, GameError, Replicant};
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, ACTOR_FALL_DISTANCE_OFFSET,
    ACTOR_PENDING_DISPLACEMENT_OFFSET, ACTOR_VERTICAL_SPEED_OFFSET,
};

const FLOAT_LANE_BYTES: u64 = 4;

impl<M: GameMemory> Replicant<M> {
    pub fn write_noclip_step(&mut self, actor: u64, step: Displacement) -> Result<(), GameError> {
        let memory = self.memory_mut();
        let displacement = actor + ACTOR_PENDING_DISPLACEMENT_OFFSET;
        memory.write_f32(displacement, step.x)?;
        memory.write_f32(displacement + FLOAT_LANE_BYTES, step.y)?;
        memory.write_f32(displacement + 2 * FLOAT_LANE_BYTES, step.z)?;
        memory.write_u8(actor + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, 1)?;
        memory.write_f32(actor + ACTOR_VERTICAL_SPEED_OFFSET, 0.0)?;
        memory.write_f32(actor + ACTOR_FALL_DISTANCE_OFFSET, 0.0)?;
        Ok(())
    }
}

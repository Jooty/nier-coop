use crate::blocks::{BLOCK_POSITION_X_OFFSET, BLOCK_POSITION_Y_OFFSET, BLOCK_POSITION_Z_OFFSET};
use crate::memory::{GameMemory, MemoryError};

pub fn read_block_position<M: GameMemory>(memory: &M, actor: u64) -> Option<[f32; 3]> {
    let position = [
        memory.read_f32(actor + BLOCK_POSITION_X_OFFSET).ok()?,
        memory.read_f32(actor + BLOCK_POSITION_Y_OFFSET).ok()?,
        memory.read_f32(actor + BLOCK_POSITION_Z_OFFSET).ok()?,
    ];
    position
        .iter()
        .all(|axis| axis.is_finite())
        .then_some(position)
}

pub fn write_block_position<M: GameMemory>(
    memory: &mut M,
    actor: u64,
    position: [f32; 3],
) -> Result<(), MemoryError> {
    memory.write_f32(actor + BLOCK_POSITION_X_OFFSET, position[0])?;
    memory.write_f32(actor + BLOCK_POSITION_Y_OFFSET, position[1])?;
    memory.write_f32(actor + BLOCK_POSITION_Z_OFFSET, position[2])
}

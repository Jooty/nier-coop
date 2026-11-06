use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::ACTOR_ANCHOR_OFFSET_OFFSET;

pub fn read_anchor_offset<M: GameMemory>(memory: &M, actor: u64) -> Result<[f32; 3], GameError> {
    let mut offset = [0.0; 3];
    for (index, value) in offset.iter_mut().enumerate() {
        *value = memory.read_f32(actor + ACTOR_ANCHOR_OFFSET_OFFSET + index as u64 * 4)?;
    }
    Ok(offset)
}

/// Gives `target` the anchor offset of `source` (the vector the game adds to an actor's
/// world matrix) and returns what `target` held before.
pub fn match_anchor_offset<M: GameMemory>(
    memory: &mut M,
    source: u64,
    target: u64,
) -> Result<[f32; 3], GameError> {
    let wanted = read_anchor_offset(memory, source)?;
    let before = read_anchor_offset(memory, target)?;
    if before != wanted {
        for (index, value) in wanted.iter().enumerate() {
            memory.write_f32(
                target + ACTOR_ANCHOR_OFFSET_OFFSET + index as u64 * 4,
                *value,
            )?;
        }
    }
    Ok(before)
}

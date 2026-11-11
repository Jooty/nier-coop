use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA,
    HANDLE_POOL_SLOT_COUNT,
};

pub fn actor_is_live<M: GameMemory>(memory: &M, actor: u64) -> Result<bool, GameError> {
    let Ok(handle) = memory.read_array::<8>(actor + ACTOR_SELF_HANDLE_OFFSET) else {
        return Ok(false);
    };
    let index = u32::from_le_bytes([handle[0], handle[1], handle[2], handle[3]]);
    let generation = u32::from_le_bytes([handle[4], handle[5], handle[6], handle[7]]);
    if index >= HANDLE_POOL_SLOT_COUNT {
        return Ok(false);
    }
    let base = memory.module_base();
    let slot_pointer =
        memory.read_u64(base + u64::from(HANDLE_POOL_POINTERS_RVA) + u64::from(index) * 8)?;
    let slot_generation =
        memory.read_i32(base + u64::from(HANDLE_POOL_GENERATIONS_RVA) + u64::from(index) * 4)?;
    Ok(slot_pointer == actor && slot_generation.cast_unsigned() == generation)
}

/// The actor a handle (pool slot, generation) names, or None once that slot was given away.
pub fn actor_of_handle<M: GameMemory>(
    memory: &M,
    index: u32,
    generation: u32,
) -> Result<Option<u64>, GameError> {
    if index >= HANDLE_POOL_SLOT_COUNT {
        return Ok(None);
    }
    let base = memory.module_base();
    let slot_generation =
        memory.read_i32(base + u64::from(HANDLE_POOL_GENERATIONS_RVA) + u64::from(index) * 4)?;
    if slot_generation.cast_unsigned() != generation {
        return Ok(None);
    }
    let actor =
        memory.read_u64(base + u64::from(HANDLE_POOL_POINTERS_RVA) + u64::from(index) * 8)?;
    Ok((actor != 0).then_some(actor))
}

use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    EFFECT_HANDLE_GENERATION_SHIFT, EFFECT_HANDLE_INDEX_MASK, EFFECT_INSTANCE_CHANGED_FLAG_OFFSET,
    EFFECT_INSTANCE_FINISHED_FLAG_OFFSET, EFFECT_INSTANCE_MATRIX_OFFSET, EFFECT_INVALID_HANDLE,
    EFFECT_POOL_GENERATION_OFFSET, EFFECT_POOL_RVA, EFFECT_POOL_SLOT_STRIDE,
};
use replicant_proto::WORLD_EFFECT_TRANSFORM_FLOATS;

/// The live effect a handle names, or `None` when the handle is invalid or its slot has been
/// reused since (the generation in the handle's high bits no longer matches).
pub fn effect_instance_address<M: GameMemory>(
    memory: &M,
    handle: u32,
) -> Result<Option<u64>, GameError> {
    if handle == EFFECT_INVALID_HANDLE {
        return Ok(None);
    }
    let slot = memory.module_base()
        + u64::from(EFFECT_POOL_RVA)
        + u64::from(handle & EFFECT_HANDLE_INDEX_MASK) * EFFECT_POOL_SLOT_STRIDE;
    let generation = memory
        .read_i32(slot + EFFECT_POOL_GENERATION_OFFSET)?
        .cast_unsigned();
    if generation != handle >> EFFECT_HANDLE_GENERATION_SHIFT {
        return Ok(None);
    }
    match memory.read_u64(slot)? {
        0 => Ok(None),
        instance => Ok(Some(instance)),
    }
}

/// Marks a live effect finished, as the game's own effect-remove command does. `false` when
/// the handle no longer names a live effect.
pub fn finish_effect<M: GameMemory>(memory: &mut M, handle: u32) -> Result<bool, GameError> {
    let Some(instance) = effect_instance_address(memory, handle)? else {
        return Ok(false);
    };
    memory.write(
        instance + EFFECT_INSTANCE_FINISHED_FLAG_OFFSET,
        &1_u16.to_le_bytes(),
    )?;
    memory.write_u8(instance + EFFECT_INSTANCE_CHANGED_FLAG_OFFSET, 1)?;
    Ok(true)
}

/// `true` once the effect is gone or marked finished (by the remove command or by itself).
pub fn effect_is_finished<M: GameMemory>(memory: &M, handle: u32) -> Result<bool, GameError> {
    let Some(instance) = effect_instance_address(memory, handle)? else {
        return Ok(true);
    };
    let flag = memory.read_array::<2>(instance + EFFECT_INSTANCE_FINISHED_FLAG_OFFSET)?;
    Ok(u16::from_le_bytes(flag) != 0)
}

/// The world transform (3x4 rows) of a live, unfinished effect; None once it is gone or finished.
pub fn read_effect_transform<M: GameMemory>(
    memory: &M,
    handle: u32,
) -> Result<Option<[f32; WORLD_EFFECT_TRANSFORM_FLOATS]>, GameError> {
    if effect_is_finished(memory, handle)? {
        return Ok(None);
    }
    let Some(instance) = effect_instance_address(memory, handle)? else {
        return Ok(None);
    };
    let bytes = memory.read_array::<{ WORLD_EFFECT_TRANSFORM_FLOATS * 4 }>(
        instance + EFFECT_INSTANCE_MATRIX_OFFSET,
    )?;
    let mut transform = [0.0_f32; WORLD_EFFECT_TRANSFORM_FLOATS];
    for (value, chunk) in transform.iter_mut().zip(bytes.as_chunks::<4>().0) {
        *value = f32::from_le_bytes(*chunk);
    }
    Ok(Some(transform))
}

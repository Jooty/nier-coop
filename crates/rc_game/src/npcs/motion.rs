use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET, ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET,
    ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET, BODY_MOTION_LAYER,
    MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET, MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET,
    MOTION_LAYER_SLOT_COUNT_OFFSET, MOTION_SLOT_FRAME_COUNT_OFFSET, MOTION_SLOT_FRAME_OFFSET,
    MOTION_SLOT_MOTION_INDEX_OFFSET, OLD_MOTION_SLOT_LOOP_FLAG_OFFSET,
    OLD_MOTION_SLOT_SPEED_OFFSET, OLD_MOTION_SLOT_STRIDE,
};
use crate::puppet::{LayerMotion, PuppetLayer, read_puppet_layer};

/// The body layer of an NPC, from whichever motion player its model uses: the newer one inside
/// the model instance (flag at `+0x8B20`), or the older one villagers mostly use, whose slots are
/// `0x48` bytes with the loop flag at `+0x10` and the speed at `+0x14`.
pub fn read_npc_body_layer<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Option<PuppetLayer>, GameError> {
    if memory.read_u8(actor + ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET)? != 0 {
        return read_puppet_layer(memory, actor, BODY_MOTION_LAYER);
    }
    let layer_count = memory.read_i32(actor + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET)?;
    if i64::from(BODY_MOTION_LAYER) >= i64::from(layer_count) {
        return Ok(None);
    }
    let layer = memory.read_u64(
        actor + ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET + u64::from(BODY_MOTION_LAYER) * 8,
    )?;
    if layer == 0 || memory.read_u64(layer)? == 0 {
        return Ok(None);
    }
    let slot_number = memory.read_i32(layer + MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET)?;
    let slot_count = memory.read_i32(layer + MOTION_LAYER_SLOT_COUNT_OFFSET)?;
    let slots = memory.read_u64(layer + MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET)?;
    if slots == 0 || !(0..slot_count).contains(&slot_number) {
        return Ok(None);
    }
    let slot = slots + slot_number.cast_unsigned() as u64 * OLD_MOTION_SLOT_STRIDE;
    let index = memory.read_i32(slot + MOTION_SLOT_MOTION_INDEX_OFFSET)?;
    if index < 0 {
        return Ok(None);
    }
    Ok(Some(PuppetLayer {
        slot,
        motion: LayerMotion {
            index,
            frame: memory.read_f32(slot + MOTION_SLOT_FRAME_OFFSET)?,
            speed: memory.read_f32(slot + OLD_MOTION_SLOT_SPEED_OFFSET)?,
            looping: memory.read_u8(slot + OLD_MOTION_SLOT_LOOP_FLAG_OFFSET)? != 0,
        },
        frame_count: memory.read_i32(slot + MOTION_SLOT_FRAME_COUNT_OFFSET)?,
    }))
}

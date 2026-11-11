use crate::game::GameError;
use crate::game::text_up_to_nul;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_MOTION_PLAYER_OFFSET, MAX_PACK_NODES_WALKED, MOTION_ENTRY_MAGIC,
    MOTION_ENTRY_NAME_OFFSET, MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET,
    MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET, MOTION_LAYER_SLOT_COUNT_OFFSET, MOTION_NAME_READ_LEN,
    MOTION_PACK_COUNT_OFFSET, MOTION_PACK_MAGIC, MOTION_PACK_OFFSET_TABLE_OFFSET,
    MOTION_PLAYER_LAYER_COUNT_OFFSET, MOTION_PLAYER_LAYER_POINTERS_OFFSET,
    MOTION_PLAYER_PACK_LIST_OFFSET, MOTION_SLOT_FRAME_COUNT_OFFSET, MOTION_SLOT_FRAME_OFFSET,
    MOTION_SLOT_LOOP_FLAG_OFFSET, MOTION_SLOT_MOTION_INDEX_OFFSET,
    MOTION_SLOT_PREVIOUS_FRAME_OFFSET, MOTION_SLOT_SPEED_OFFSET, MOTION_SLOT_STRIDE,
    PACK_LIST_FIRST_NODE_OFFSET, PACK_NODE_NEXT_OFFSET, PACK_NODE_PACK_POINTER_OFFSET,
};
use replicant_proto::motion_name_hash;

const MAX_MOTIONS_PER_PACK: u32 = 4096;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerMotion {
    pub index: i32,
    pub frame: f32,
    pub speed: f32,
    pub looping: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionEntry {
    pub index: i32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PuppetLayer {
    pub slot: u64,
    pub motion: LayerMotion,
    pub frame_count: i32,
}

pub fn read_layer_motion<M: GameMemory>(
    memory: &M,
    actor: u64,
    layer: u32,
) -> Result<Option<LayerMotion>, GameError> {
    Ok(read_puppet_layer(memory, actor, layer)?.map(|layer| layer.motion))
}

/// The current slot of `layer` with its address and frame count, for seeking it.
pub fn read_puppet_layer<M: GameMemory>(
    memory: &M,
    actor: u64,
    layer: u32,
) -> Result<Option<PuppetLayer>, GameError> {
    let player = actor + ACTOR_MOTION_PLAYER_OFFSET;
    let layer_count = memory.read_i32(player + MOTION_PLAYER_LAYER_COUNT_OFFSET)?;
    if i64::from(layer) >= i64::from(layer_count) {
        return Ok(None);
    }
    let layer_pointer =
        memory.read_u64(player + MOTION_PLAYER_LAYER_POINTERS_OFFSET + u64::from(layer) * 8)?;
    if layer_pointer == 0 {
        return Ok(None);
    }
    let slot_number = memory.read_i32(layer_pointer + MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET)?;
    let slot_count = memory.read_i32(layer_pointer + MOTION_LAYER_SLOT_COUNT_OFFSET)?;
    let slots = memory.read_u64(layer_pointer + MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET)?;
    if slots == 0 || !(0..slot_count).contains(&slot_number) {
        return Ok(None);
    }
    let slot = slots + slot_number.cast_unsigned() as u64 * MOTION_SLOT_STRIDE;
    let index = memory.read_i32(slot + MOTION_SLOT_MOTION_INDEX_OFFSET)?;
    if index < 0 {
        return Ok(None);
    }
    Ok(Some(PuppetLayer {
        slot,
        motion: LayerMotion {
            index,
            frame: memory.read_f32(slot + MOTION_SLOT_FRAME_OFFSET)?,
            speed: memory.read_f32(slot + MOTION_SLOT_SPEED_OFFSET)?,
            looping: memory.read_u8(slot + MOTION_SLOT_LOOP_FLAG_OFFSET)? != 0,
        },
        frame_count: memory.read_i32(slot + MOTION_SLOT_FRAME_COUNT_OFFSET)?,
    }))
}

/// Writes `frame` as the slot's current and previous frame, as the game's seek does.
pub fn seek_layer_frame<M: GameMemory>(
    memory: &mut M,
    layer: &PuppetLayer,
    frame: f32,
) -> Result<(), GameError> {
    memory.write_f32(layer.slot + MOTION_SLOT_FRAME_OFFSET, frame)?;
    memory.write_f32(layer.slot + MOTION_SLOT_PREVIOUS_FRAME_OFFSET, frame)?;
    Ok(())
}

pub fn motion_entries<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Vec<MotionEntry>, GameError> {
    let list = actor + ACTOR_MOTION_PLAYER_OFFSET + MOTION_PLAYER_PACK_LIST_OFFSET;
    let mut node = memory.read_u64(list + PACK_LIST_FIRST_NODE_OFFSET)?;
    let mut entries = Vec::new();
    let mut first_index = 0_i32;
    for _ in 0..MAX_PACK_NODES_WALKED {
        if node == 0 {
            break;
        }
        let pack = memory.read_u64(node + PACK_NODE_PACK_POINTER_OFFSET)?;
        if memory.read_array::<4>(pack)? != MOTION_PACK_MAGIC {
            break;
        }
        let count = u32::from_le_bytes(memory.read_array(pack + MOTION_PACK_COUNT_OFFSET)?);
        for entry in 0..count.min(MAX_MOTIONS_PER_PACK) {
            if let Some(name) = entry_name(memory, pack, entry)? {
                entries.push(MotionEntry {
                    index: first_index + entry.cast_signed(),
                    name,
                });
            }
        }
        first_index = first_index.saturating_add(count.cast_signed());
        node = memory.read_u64(node + PACK_NODE_NEXT_OFFSET)?;
    }
    Ok(entries)
}

pub fn motion_entry_for_hash(entries: &[MotionEntry], hash: u32) -> Option<&MotionEntry> {
    entries
        .iter()
        .find(|entry| motion_name_hash(&entry.name) == hash)
}

pub fn motion_name<M: GameMemory>(
    memory: &M,
    actor: u64,
    index: i32,
) -> Result<Option<String>, GameError> {
    let Ok(mut remaining) = u32::try_from(index) else {
        return Ok(None);
    };
    let list = actor + ACTOR_MOTION_PLAYER_OFFSET + MOTION_PLAYER_PACK_LIST_OFFSET;
    let mut node = memory.read_u64(list + PACK_LIST_FIRST_NODE_OFFSET)?;
    for _ in 0..MAX_PACK_NODES_WALKED {
        if node == 0 {
            return Ok(None);
        }
        let pack = memory.read_u64(node + PACK_NODE_PACK_POINTER_OFFSET)?;
        if memory.read_array::<4>(pack)? != MOTION_PACK_MAGIC {
            return Ok(None);
        }
        let count = u32::from_le_bytes(memory.read_array(pack + MOTION_PACK_COUNT_OFFSET)?);
        if remaining < count {
            return entry_name(memory, pack, remaining);
        }
        remaining -= count;
        node = memory.read_u64(node + PACK_NODE_NEXT_OFFSET)?;
    }
    Ok(None)
}

fn entry_name<M: GameMemory>(
    memory: &M,
    pack: u64,
    entry: u32,
) -> Result<Option<String>, GameError> {
    let offset = u32::from_le_bytes(
        memory.read_array(pack + MOTION_PACK_OFFSET_TABLE_OFFSET + u64::from(entry) * 4)?,
    );
    let motion = pack + u64::from(offset);
    if memory.read_array::<3>(motion)? != MOTION_ENTRY_MAGIC {
        return Ok(None);
    }
    let bytes = memory.read_array::<MOTION_NAME_READ_LEN>(motion + MOTION_ENTRY_NAME_OFFSET)?;
    Ok(Some(text_up_to_nul(&bytes)))
}

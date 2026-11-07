use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET, ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET,
    ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET, BODY_MOTION_LAYER, MAX_OLD_MOTION_LAYERS_CHECKED,
    MOTION_LAYER_CURRENT_SLOT_INDEX_OFFSET, MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET,
    MOTION_LAYER_SLOT_COUNT_OFFSET, MOTION_RECORD_BONE_COUNT_OFFSET,
    MOTION_SLOT_FRAME_COUNT_OFFSET, MOTION_SLOT_FRAME_OFFSET, MOTION_SLOT_MOTION_INDEX_OFFSET,
    OLD_MOTION_SLOT_LOOP_FLAG_OFFSET, OLD_MOTION_SLOT_RECORD_POINTER_OFFSET,
    OLD_MOTION_SLOT_SPEED_OFFSET, OLD_MOTION_SLOT_STRIDE, OLD_MOTION_SLOT_WEIGHT_OFFSET,
};
use crate::puppet::{LayerMotion, PuppetLayer, read_puppet_layer};

const MAX_SLOTS_CHECKED_PER_LAYER: i32 = 16;
const MAX_ENEMY_LAYERS_SEARCHED: u32 = 6;

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
    read_old_player_layer(memory, actor, BODY_MOTION_LAYER)
}

/// An enemy's playing body motion: layer 0 of either player first, then the other layers, as
/// the game's own getter (RVA 0x154ED0) falls back from the newer player to the older one.
pub fn read_enemy_body_layer<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Option<PuppetLayer>, GameError> {
    let new_player = memory.read_u8(actor + ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET)? != 0;
    for layer_number in 0..MAX_ENEMY_LAYERS_SEARCHED {
        if new_player && let Some(layer) = read_puppet_layer(memory, actor, layer_number)? {
            return Ok(Some(layer));
        }
        if let Some(layer) = read_old_player_layer(memory, actor, layer_number)? {
            return Ok(Some(layer));
        }
    }
    Ok(None)
}

pub fn motion_layers_text<M: GameMemory>(memory: &M, actor: u64) -> String {
    let new_player = memory.read_u8(actor + ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET);
    let old_count = memory.read_i32(actor + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET);
    let layers: Vec<String> = (0..MAX_ENEMY_LAYERS_SEARCHED)
        .map(|layer_number| {
            let new = layer_text(read_puppet_layer(memory, actor, layer_number));
            let old = layer_text(read_old_player_layer(memory, actor, layer_number));
            format!("layer {layer_number} new {new} old {old}")
        })
        .collect();
    format!(
        "new player flag {new_player:?}, old layer count {old_count:?}, {}",
        layers.join(", ")
    )
}

fn layer_text(layer: Result<Option<PuppetLayer>, GameError>) -> String {
    match layer {
        Ok(Some(layer)) => format!("motion {}", layer.motion.index),
        Ok(None) => "none".to_owned(),
        Err(_) => "unreadable".to_owned(),
    }
}

fn read_old_player_layer<M: GameMemory>(
    memory: &M,
    actor: u64,
    layer_number: u32,
) -> Result<Option<PuppetLayer>, GameError> {
    let layer_count = memory.read_i32(actor + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET)?;
    if i64::from(layer_number) >= i64::from(layer_count) {
        return Ok(None);
    }
    let layer = memory.read_u64(
        actor + ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET + u64::from(layer_number) * 8,
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

/// False when a villager's older motion player holds a playing slot whose motion record cannot
/// be read; the game's pose code (RVA 0x3FE7E0) faults on it. The newer player is not checked.
pub fn npc_motion_records_readable<M: GameMemory>(memory: &M, actor: u64) -> bool {
    if memory
        .read_u8(actor + ACTOR_NEW_MOTION_PLAYER_FLAG_OFFSET)
        .is_ok_and(|flag| flag != 0)
    {
        return true;
    }
    let Ok(layer_count) = memory.read_i32(actor + ACTOR_OLD_MOTION_LAYER_COUNT_OFFSET) else {
        return false;
    };
    (0..layer_count.clamp(0, MAX_OLD_MOTION_LAYERS_CHECKED)).all(|layer_number| {
        layer_records_readable(memory, actor, layer_number.cast_unsigned().into())
    })
}

fn layer_records_readable<M: GameMemory>(memory: &M, actor: u64, layer_number: u64) -> bool {
    let Ok(layer) =
        memory.read_u64(actor + ACTOR_OLD_MOTION_LAYER_POINTERS_OFFSET + layer_number * 8)
    else {
        return false;
    };
    if layer == 0 {
        return true;
    }
    let (Ok(slot_count), Ok(slots)) = (
        memory.read_i32(layer + MOTION_LAYER_SLOT_COUNT_OFFSET),
        memory.read_u64(layer + MOTION_LAYER_SLOT_ARRAY_POINTER_OFFSET),
    ) else {
        return false;
    };
    if slots == 0 {
        return true;
    }
    (0..slot_count.clamp(0, MAX_SLOTS_CHECKED_PER_LAYER)).all(|slot_number| {
        let slot = slots + slot_number.cast_unsigned() as u64 * OLD_MOTION_SLOT_STRIDE;
        slot_record_readable(memory, slot)
    })
}

fn slot_record_readable<M: GameMemory>(memory: &M, slot: u64) -> bool {
    let (Ok(index), Ok(weight)) = (
        memory.read_i32(slot + MOTION_SLOT_MOTION_INDEX_OFFSET),
        memory.read_f32(slot + OLD_MOTION_SLOT_WEIGHT_OFFSET),
    ) else {
        return false;
    };
    if index < 0 || weight <= 0.0 {
        return true;
    }
    match memory.read_u64(slot + OLD_MOTION_SLOT_RECORD_POINTER_OFFSET) {
        Ok(0) => true,
        Ok(record) => memory
            .read_array::<2>(record + MOTION_RECORD_BONE_COUNT_OFFSET)
            .is_ok(),
        Err(_) => false,
    }
}

use crate::marker::{CONTROL_MASK_FIRST_RVA, CONTROL_MASK_SLOT_COUNT, EVENT_SCENE_BYTE_RVA};
use crate::memory::GameMemory;
use crate::move_log::{
    APPLIED_STEP_OFFSET, COLLISION_FLAGS_OFFSET, CURRENT_STATE_POINTER_OFFSET, DAMAGE_MODE_OFFSET,
    DISPLACEMENT_PENDING_FLAG_OFFSET, GROUND_VELOCITY_OFFSET, GROUND_VELOCITY_Z_OFFSET,
    LOCK_BYTE_FLAGS_OFFSET, MOVE_BLOCK_OFFSET, MOVE_LOG_RATE_CHANNEL_COUNT, MOVE_LOG_RATE_ID_COUNT,
    MOVE_MODE_OFFSET, RATE_TABLE_CHANNEL_STRIDE, RATE_TABLE_ID_STRIDE, RATE_TABLE_OFFSET,
    STATE_LEVEL_OFFSET, STATE_LIMIT_OFFSET, STATE_LOCK_OFFSET, STEPS_PER_SECOND, read_move_keys,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RateChange {
    pub id: usize,
    pub channel: usize,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MoveSample {
    pub state_rva: Option<u32>,
    pub speed: Option<f32>,
    pub applied_step_per_second: Option<f32>,
    pub displacement_pending: Option<u8>,
    pub state_lock: Option<i32>,
    pub state_level: Option<i32>,
    pub state_limit: Option<i32>,
    pub move_block: Option<i32>,
    pub damage_mode: Option<i32>,
    pub collision_flags: Option<[u8; 2]>,
    pub lock_byte_flags: Option<[u8; 2]>,
    pub rate_changes: Option<Vec<RateChange>>,
    pub mask_slots: Option<[u32; 4]>,
    pub event_scene: Option<bool>,
    pub move_keys: Option<[bool; 4]>,
    pub move_axis: Option<[f32; 2]>,
    pub ground_x_z: Option<[f32; 2]>,
    pub move_mode: Option<i32>,
}

pub fn read_move_sample<M: GameMemory>(memory: &M, actor: u64) -> MoveSample {
    let base = memory.module_base();
    MoveSample {
        state_rva: memory
            .read_u64(actor + CURRENT_STATE_POINTER_OFFSET)
            .ok()
            .filter(|state| *state != 0)
            .and_then(|state| memory.read_u64(state).ok())
            .and_then(|vftable| u32::try_from(vftable.checked_sub(base)?).ok()),
        speed: read_length(memory, actor + GROUND_VELOCITY_OFFSET, 3),
        applied_step_per_second: read_length(memory, actor + APPLIED_STEP_OFFSET, 3)
            .map(|step| step * STEPS_PER_SECOND),
        displacement_pending: memory
            .read_u8(actor + DISPLACEMENT_PENDING_FLAG_OFFSET)
            .ok(),
        state_lock: memory.read_i32(actor + STATE_LOCK_OFFSET).ok(),
        state_level: memory.read_i32(actor + STATE_LEVEL_OFFSET).ok(),
        state_limit: memory.read_i32(actor + STATE_LIMIT_OFFSET).ok(),
        move_block: memory.read_i32(actor + MOVE_BLOCK_OFFSET).ok(),
        damage_mode: memory.read_i32(actor + DAMAGE_MODE_OFFSET).ok(),
        collision_flags: memory.read_array(actor + COLLISION_FLAGS_OFFSET).ok(),
        lock_byte_flags: memory.read_array(actor + LOCK_BYTE_FLAGS_OFFSET).ok(),
        rate_changes: read_rate_changes(memory, actor),
        mask_slots: read_mask_slots(memory),
        event_scene: memory
            .read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))
            .ok()
            .map(|byte| byte != 0),
        move_keys: read_move_keys(memory),
        move_axis: None,
        ground_x_z: read_ground_x_z(memory, actor),
        move_mode: memory.read_i32(actor + MOVE_MODE_OFFSET).ok(),
    }
}

fn read_length<M: GameMemory>(memory: &M, address: u64, lanes: u64) -> Option<f32> {
    let mut sum = 0.0_f32;
    for lane in 0..lanes {
        let value = memory.read_f32(address + lane * 4).ok()?;
        sum += value * value;
    }
    Some(sum.sqrt())
}

fn read_ground_x_z<M: GameMemory>(memory: &M, actor: u64) -> Option<[f32; 2]> {
    Some([
        memory.read_f32(actor + GROUND_VELOCITY_OFFSET).ok()?,
        memory.read_f32(actor + GROUND_VELOCITY_Z_OFFSET).ok()?,
    ])
}

fn read_mask_slots<M: GameMemory>(memory: &M) -> Option<[u32; 4]> {
    let first = memory.module_base() + u64::from(CONTROL_MASK_FIRST_RVA);
    let mut slots = [0_u32; 4];
    for (slot, value) in (0..CONTROL_MASK_SLOT_COUNT).zip(slots.iter_mut()) {
        *value = u32::from_le_bytes(memory.read_array(first + u64::from(slot) * 4).ok()?);
    }
    Some(slots)
}

fn read_rate_changes<M: GameMemory>(memory: &M, actor: u64) -> Option<Vec<RateChange>> {
    let mut table = vec![0_u8; MOVE_LOG_RATE_ID_COUNT * RATE_TABLE_ID_STRIDE];
    memory.read(actor + RATE_TABLE_OFFSET, &mut table).ok()?;
    let mut changes = Vec::new();
    for id in 0..MOVE_LOG_RATE_ID_COUNT {
        for channel in 0..MOVE_LOG_RATE_CHANNEL_COUNT {
            let at = id * RATE_TABLE_ID_STRIDE + channel * RATE_TABLE_CHANNEL_STRIDE;
            let value =
                f32::from_le_bytes([table[at], table[at + 1], table[at + 2], table[at + 3]]);
            if (value - 1.0).abs() > f32::EPSILON {
                changes.push(RateChange { id, channel, value });
            }
        }
    }
    Some(changes)
}

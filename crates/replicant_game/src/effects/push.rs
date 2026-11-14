use std::collections::VecDeque;

use replicant_proto::WORLD_EFFECT_TRANSFORM_FLOATS;

use crate::effects::capture::read_text;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    EFFECT_NAME_ENTRY_FIRST_OFFSET, EFFECT_NAME_ENTRY_ID_OFFSET, EFFECT_NAME_ENTRY_STRIDE,
    EFFECT_NAME_READ_LEN, EFFECT_NAME_TABLES_OFFSET, EFFECT_PUSH_BY_NAME_RVA_RANGE,
    EFFECT_PUSH_FLUSHER_A_RVA_RANGE, EFFECT_PUSH_FLUSHER_B_RVA_RANGE, EFFECT_REGISTRY_RVA,
    EFFECT_REQUEST_GENERATION_OFFSET, EFFECT_REQUEST_INDEX_OFFSET, EFFECT_REQUEST_MATRIX_OFFSET,
};

pub const AFTER_OWN_HIT_WINDOW_MS: u64 = 250;
pub const MAX_RECENT_PUSHED_HANDLES: usize = 64;
pub const FLUSHER_HIT_RADIUS: f32 = 4.0;
pub const DROP_LOG_INTERVAL_MS: u64 = 1000;
const POINTER_BYTES: u64 = 8;
const FLOAT_BYTES: usize = 4;
const MATRIX_BYTES: usize = WORLD_EFFECT_TRANSFORM_FLOATS * FLOAT_BYTES;
const NAME_ID_INDEX_MASK: u32 = 0xFFFF;
const NAME_ID_GENERATION_SHIFT: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PushCaller {
    ByName,
    FlusherA,
    FlusherB,
    Other,
}

impl PushCaller {
    pub fn label(self) -> &'static str {
        match self {
            Self::ByName => "by-name",
            Self::FlusherA => "flusher-a",
            Self::FlusherB => "flusher-b",
            Self::Other => "other",
        }
    }

    pub fn is_flusher(self) -> bool {
        matches!(self, Self::FlusherA | Self::FlusherB)
    }
}

pub fn classify_push_caller(caller_rva: u32) -> PushCaller {
    if EFFECT_PUSH_BY_NAME_RVA_RANGE.contains(&caller_rva) {
        PushCaller::ByName
    } else if EFFECT_PUSH_FLUSHER_A_RVA_RANGE.contains(&caller_rva) {
        PushCaller::FlusherA
    } else if EFFECT_PUSH_FLUSHER_B_RVA_RANGE.contains(&caller_rva) {
        PushCaller::FlusherB
    } else {
        PushCaller::Other
    }
}

pub fn within_after_hit_window(ms_since_own_hit: Option<u64>) -> bool {
    ms_since_own_hit.is_some_and(|ms| ms <= AFTER_OWN_HIT_WINDOW_MS)
}

pub fn within_hit_radius(spawn: [f32; 3], hit: Option<[f32; 3]>) -> bool {
    hit.is_some_and(|hit| {
        let squared: f32 = spawn
            .iter()
            .zip(hit)
            .map(|(spawn, hit)| (spawn - hit) * (spawn - hit))
            .sum();
        squared <= FLUSHER_HIT_RADIUS * FLUSHER_HIT_RADIUS
    })
}

pub fn push_is_sent(
    caller: PushCaller,
    in_flight: bool,
    ms_since_own_hit: Option<u64>,
    near_hit: bool,
) -> bool {
    caller.is_flusher() && near_hit && (in_flight || within_after_hit_window(ms_since_own_hit))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingAdmission {
    Append,
    ReplaceAt(usize),
    Drop,
}

pub fn admit_pending(
    queued_in_hit: &[bool],
    new_in_hit: bool,
    capacity: usize,
) -> PendingAdmission {
    if queued_in_hit.len() < capacity {
        return PendingAdmission::Append;
    }
    if !new_in_hit {
        return PendingAdmission::Drop;
    }
    queued_in_hit
        .iter()
        .rposition(|in_hit| !in_hit)
        .map_or(PendingAdmission::Drop, PendingAdmission::ReplaceAt)
}

pub fn census_log_line(
    name: &str,
    caller: PushCaller,
    caller_rva: u32,
    ms_since_own_hit: Option<u64>,
    in_flight: bool,
) -> String {
    let hit_ms = ms_since_own_hit.map_or_else(|| "none".to_owned(), |ms| ms.to_string());
    format!(
        "effects: effect census {name} from {} {caller_rva:#x} hit_ms={hit_ms} in_flight={in_flight}",
        caller.label()
    )
}

#[derive(Debug, Default)]
pub struct RecentHandles {
    handles: VecDeque<u32>,
}

impl RecentHandles {
    pub fn first_time(&mut self, handle: u32) -> bool {
        if self.handles.contains(&handle) {
            return false;
        }
        if self.handles.len() >= MAX_RECENT_PUSHED_HANDLES {
            self.handles.pop_front();
        }
        self.handles.push_back(handle);
        true
    }
}

pub fn read_request_id<M: GameMemory>(memory: &M, request: u64) -> Result<u32, GameError> {
    let index = memory
        .read_i32(request + EFFECT_REQUEST_INDEX_OFFSET)?
        .cast_unsigned();
    let generation = memory
        .read_i32(request + EFFECT_REQUEST_GENERATION_OFFSET)?
        .cast_unsigned();
    Ok((index & NAME_ID_INDEX_MASK) | (generation << NAME_ID_GENERATION_SHIFT))
}

pub fn read_request_name<M: GameMemory>(
    memory: &M,
    request: u64,
) -> Result<Option<String>, GameError> {
    let id = read_request_id(memory, request)?;
    let index = id & NAME_ID_INDEX_MASK;
    let generation = u64::from(id >> NAME_ID_GENERATION_SHIFT);
    if index == NAME_ID_INDEX_MASK {
        return Ok(None);
    }
    let registry = memory.module_base() + u64::from(EFFECT_REGISTRY_RVA);
    let table =
        memory.read_u64(registry + EFFECT_NAME_TABLES_OFFSET + u64::from(index) * POINTER_BYTES)?;
    if table == 0 || i64::from(memory.read_i32(table)?) <= generation.cast_signed() {
        return Ok(None);
    }
    let entry = table + EFFECT_NAME_ENTRY_FIRST_OFFSET + generation * EFFECT_NAME_ENTRY_STRIDE;
    if memory
        .read_i32(entry + EFFECT_NAME_ENTRY_ID_OFFSET)?
        .cast_unsigned()
        != id
    {
        return Ok(None);
    }
    let name = memory.read_u64(entry)?;
    if name == 0 {
        return Ok(None);
    }
    read_text(memory, name, EFFECT_NAME_READ_LEN)
}

pub fn read_request_transform<M: GameMemory>(
    memory: &M,
    request: u64,
) -> Result<[f32; WORLD_EFFECT_TRANSFORM_FLOATS], GameError> {
    let bytes = memory.read_array::<MATRIX_BYTES>(request + EFFECT_REQUEST_MATRIX_OFFSET)?;
    Ok(core::array::from_fn(|index| {
        let at = index * FLOAT_BYTES;
        f32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }))
}

use std::collections::HashSet;

use crate::effects::capture::read_text;
use crate::game::GameError;
use crate::memory::GameMemory;
use crate::offsets::{
    CHARACTER_RECORD_NAME_OFFSET, CHARACTER_STATE_READ_LEN, PLAYER_CHARACTER_RECORD_OFFSET,
};

pub const VOICE_SWITCH_GROUP: &str = "chr";
const WWISE_HASH_BASIS: u32 = 0x811C_9DC5;
const WWISE_HASH_PRIME: u32 = 0x0100_0193;
const VOICE_CHARACTER_RESEND_MS: u64 = 3000;
const MAX_REMEMBERED_NAMES: usize = 256;

#[derive(Debug, Default)]
pub struct VoiceCharacterSchedule {
    last_sent: Option<(String, u64)>,
}

#[derive(Debug, Default)]
pub struct FirstTimeNames {
    seen: HashSet<String>,
}

impl VoiceCharacterSchedule {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_due(&mut self, state: &str, now_ms: u64) -> bool {
        let due = match &self.last_sent {
            Some((sent, at)) => {
                sent != state || now_ms.saturating_sub(*at) >= VOICE_CHARACTER_RESEND_MS
            }
            None => true,
        };
        if due {
            self.last_sent = Some((state.to_owned(), now_ms));
        }
        due
    }

    pub fn reset(&mut self) {
        self.last_sent = None;
    }
}

impl FirstTimeNames {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_first(&mut self, name: &str) -> bool {
        if self.seen.contains(name) || self.seen.len() >= MAX_REMEMBERED_NAMES {
            return false;
        }
        self.seen.insert(name.to_owned())
    }
}

pub fn wwise_name_hash(name: &str) -> u32 {
    name.bytes().fold(WWISE_HASH_BASIS, |hash, byte| {
        hash.wrapping_mul(WWISE_HASH_PRIME) ^ u32::from(byte.to_ascii_lowercase())
    })
}

pub fn read_character_state<M: GameMemory>(
    memory: &M,
    actor: u64,
) -> Result<Option<String>, GameError> {
    let record = memory.read_u64(actor + PLAYER_CHARACTER_RECORD_OFFSET)?;
    if record == 0 {
        return Ok(None);
    }
    let name_field = record + CHARACTER_RECORD_NAME_OFFSET;
    let relative = memory.read_i32(name_field)?;
    if relative == 0 {
        return Ok(None);
    }
    let name_address = name_field.wrapping_add_signed(i64::from(relative));
    read_text(memory, name_address, CHARACTER_STATE_READ_LEN)
}

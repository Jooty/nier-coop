use serde::{Deserialize, Serialize};

use crate::enemies::{
    GENERATOR_ACTION_ID_OFFSET, GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET, GENERATOR_HEADER_LEN,
    GENERATOR_MOST_ENEMIES_ALIVE_OFFSET, GENERATOR_NEXT_POINTER_OFFSET,
    GENERATOR_SCREEN_TEST_KIND_OFFSET, GENERATOR_SET_COUNT_OFFSET, GENERATOR_SET_LEN,
    GENERATOR_SET_OBJECT_ID_OFFSET, GENERATOR_SETS_POINTER_OFFSET, GENERATOR_SLOT_COUNT_OFFSET,
    GENERATOR_SLOT_ENTITY_HANDLE_OFFSET, GENERATOR_SLOT_LEN, GENERATOR_SLOT_POSITION_OFFSET,
    GENERATOR_SLOT_ROTATION_OFFSET, GENERATOR_SLOT_SET_INDEX_OFFSET,
    GENERATOR_SLOT_STATE_ENEMY_MADE, GENERATOR_SLOT_STATE_NEW, GENERATOR_SLOT_STATE_OFFSET,
    GENERATOR_SLOTS_POINTER_OFFSET, GENERATOR_SPAWNED_COUNT_OFFSET, GENERATOR_STATE_HASH_FINISHED,
    GENERATOR_STATE_HASH_OFFSET, GENERATOR_STATE_HASH_RESTING, GENERATOR_STATE_HASH_SPAWNING,
    GENERATOR_STATE_HASH_SWITCHED_OFF, GENERATOR_STATE_HASH_WAITING,
    GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET,
};

/// The fields of one enemy generator (`hap::EnemyGenerator`) the spawn share reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratorHeader {
    pub action_id: u32,
    pub state_hash: u32,
    pub spawned: u64,
    pub slots: u64,
    pub slot_count: u64,
    pub enemies_of_one_run: i32,
    pub most_enemies_alive: i32,
    pub sets: u64,
    pub set_count: u64,
    pub screen_test_kind: i32,
    pub uses_the_second_set_values: bool,
    pub next: u64,
}

/// Where a generator stands, from the state hash its action carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GeneratorPhase {
    Waiting,
    Spawning,
    Resting,
    Finished,
    SwitchedOff,
    Other,
}

/// One entry of a generator's own list of the enemies it is loading or has made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratorSlot {
    pub state: i32,
    pub set_index: u32,
    pub handle: u32,
    pub position: [f32; 4],
    pub rotation: [f32; 4],
}

impl GeneratorHeader {
    pub fn phase(&self) -> GeneratorPhase {
        GeneratorPhase::of_state_hash(self.state_hash)
    }
}

impl GeneratorPhase {
    pub fn of_state_hash(state_hash: u32) -> Self {
        match state_hash {
            GENERATOR_STATE_HASH_WAITING => Self::Waiting,
            GENERATOR_STATE_HASH_SPAWNING => Self::Spawning,
            GENERATOR_STATE_HASH_RESTING => Self::Resting,
            GENERATOR_STATE_HASH_FINISHED => Self::Finished,
            GENERATOR_STATE_HASH_SWITCHED_OFF => Self::SwitchedOff,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Waiting => "waiting",
            Self::Spawning => "spawning",
            Self::Resting => "resting before its next run",
            Self::Finished => "finished",
            Self::SwitchedOff => "switched off",
            Self::Other => "in a state this mod does not know",
        }
    }

    /// The generator has made every enemy of its run and all of them are defeated.
    pub fn is_after_a_run(self) -> bool {
        matches!(self, Self::Resting | Self::Finished)
    }
}

impl GeneratorSlot {
    pub fn holds_a_made_enemy(&self) -> bool {
        self.state == GENERATOR_SLOT_STATE_ENEMY_MADE
    }

    pub fn waits_for_its_enemy(&self) -> bool {
        (GENERATOR_SLOT_STATE_NEW..GENERATOR_SLOT_STATE_ENEMY_MADE).contains(&self.state)
    }

    /// The same set and the same bits of position and rotation: what a slot made from a spawn keeps.
    pub fn was_made_for(&self, set_index: u32, position: &[f32; 4], rotation: &[f32; 4]) -> bool {
        self.set_index == set_index
            && same_bits(&self.position, position)
            && same_bits(&self.rotation, rotation)
    }
}

pub fn generator_header(bytes: &[u8]) -> Option<GeneratorHeader> {
    if bytes.len() < GENERATOR_HEADER_LEN {
        return None;
    }
    Some(GeneratorHeader {
        action_id: u32_at(bytes, GENERATOR_ACTION_ID_OFFSET)?,
        state_hash: u32_at(bytes, GENERATOR_STATE_HASH_OFFSET)?,
        spawned: u64_at(bytes, GENERATOR_SPAWNED_COUNT_OFFSET)?,
        slots: u64_at(bytes, GENERATOR_SLOTS_POINTER_OFFSET)?,
        slot_count: u64_at(bytes, GENERATOR_SLOT_COUNT_OFFSET)?,
        enemies_of_one_run: u32_at(bytes, GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET)? as i32,
        most_enemies_alive: u32_at(bytes, GENERATOR_MOST_ENEMIES_ALIVE_OFFSET)? as i32,
        sets: u64_at(bytes, GENERATOR_SETS_POINTER_OFFSET)?,
        set_count: u64_at(bytes, GENERATOR_SET_COUNT_OFFSET)?,
        screen_test_kind: u32_at(bytes, GENERATOR_SCREEN_TEST_KIND_OFFSET)? as i32,
        uses_the_second_set_values: *bytes.get(GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET)? != 0,
        next: u64_at(bytes, GENERATOR_NEXT_POINTER_OFFSET)?,
    })
}

/// Every whole slot of a generator's slot bytes, in the game's order.
pub fn generator_slots(bytes: &[u8]) -> Vec<GeneratorSlot> {
    bytes
        .as_chunks::<GENERATOR_SLOT_LEN>()
        .0
        .iter()
        .filter_map(|slot| {
            Some(GeneratorSlot {
                state: u32_at(slot, GENERATOR_SLOT_STATE_OFFSET)? as i32,
                set_index: u32_at(slot, GENERATOR_SLOT_SET_INDEX_OFFSET)?,
                handle: u32_at(slot, GENERATOR_SLOT_ENTITY_HANDLE_OFFSET)?,
                position: f32s_at(slot, GENERATOR_SLOT_POSITION_OFFSET)?,
                rotation: f32s_at(slot, GENERATOR_SLOT_ROTATION_OFFSET)?,
            })
        })
        .collect()
}

/// The record the game's own spawn decision hands to the slot list: a new slot with no enemy yet.
pub fn new_slot_bytes(
    set_index: u32,
    position: &[f32; 4],
    rotation: &[f32; 4],
) -> [u8; GENERATOR_SLOT_LEN] {
    let mut bytes = [0_u8; GENERATOR_SLOT_LEN];
    bytes[GENERATOR_SLOT_STATE_OFFSET..GENERATOR_SLOT_STATE_OFFSET + 4]
        .copy_from_slice(&GENERATOR_SLOT_STATE_NEW.to_le_bytes());
    bytes[GENERATOR_SLOT_SET_INDEX_OFFSET..GENERATOR_SLOT_SET_INDEX_OFFSET + 4]
        .copy_from_slice(&set_index.to_le_bytes());
    for (index, value) in position.iter().enumerate() {
        let at = GENERATOR_SLOT_POSITION_OFFSET + index * 4;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    for (index, value) in rotation.iter().enumerate() {
        let at = GENERATOR_SLOT_ROTATION_OFFSET + index * 4;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

pub fn object_id_of_set(sets: &[u8], set_index: u32) -> Option<u32> {
    let start = (set_index as usize).checked_mul(GENERATOR_SET_LEN)?;
    let set = sets.get(start..start.checked_add(GENERATOR_SET_LEN)?)?;
    u32_at(set, GENERATOR_SET_OBJECT_ID_OFFSET)
}

fn same_bits(left: &[f32; 4], right: &[f32; 4]) -> bool {
    left.iter()
        .zip(right)
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let found = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([found[0], found[1], found[2], found[3]]))
}

fn u64_at(bytes: &[u8], offset: usize) -> Option<u64> {
    let low = u32_at(bytes, offset)?;
    let high = u32_at(bytes, offset + 4)?;
    Some(u64::from(high) << 32 | u64::from(low))
}

fn f32s_at(bytes: &[u8], offset: usize) -> Option<[f32; 4]> {
    let mut values = [0.0_f32; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = f32::from_bits(u32_at(bytes, offset + index * 4)?);
    }
    Some(values)
}

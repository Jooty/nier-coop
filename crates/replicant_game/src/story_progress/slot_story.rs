use crate::offsets::PHASE_NAME_BUFFER_LEN;
use crate::story_progress::{
    PHASE_NAME_BLOCK_OFFSET, PLAYER_NAME_BLOCK_OFFSET, PLAYER_NAME_BYTES, SAVE_POINT_BLOCK_OFFSET,
    SLOT_CHECKSUM_COVERED_BYTES, SLOT_CHECKSUM_OFFSET, StoryArrays, StoryRank,
};

const SAVE_POINT_BYTES: usize = 4;

/// The story part of a save slot or of the live save block: the four arrays, the phase name and
/// the save point. Everything else in a slot is the player's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotStory {
    pub arrays: StoryArrays,
    pub phase_name: [u8; PHASE_NAME_BUFFER_LEN],
    pub save_point: [u8; SAVE_POINT_BYTES],
}

impl SlotStory {
    pub fn read(slot: &[u8]) -> Option<Self> {
        let arrays = StoryArrays::from_block(slot)?;
        let phase_name = slot
            .get(PHASE_NAME_BLOCK_OFFSET..PHASE_NAME_BLOCK_OFFSET + PHASE_NAME_BUFFER_LEN)?
            .try_into()
            .ok()?;
        let save_point = slot
            .get(SAVE_POINT_BLOCK_OFFSET..SAVE_POINT_BLOCK_OFFSET + SAVE_POINT_BYTES)?
            .try_into()
            .ok()?;
        Some(Self {
            arrays,
            phase_name,
            save_point,
        })
    }

    pub fn write(&self, slot: &mut [u8]) -> bool {
        let Some(phase) =
            slot.get_mut(PHASE_NAME_BLOCK_OFFSET..PHASE_NAME_BLOCK_OFFSET + PHASE_NAME_BUFFER_LEN)
        else {
            return false;
        };
        phase.copy_from_slice(&self.phase_name);
        let Some(point) =
            slot.get_mut(SAVE_POINT_BLOCK_OFFSET..SAVE_POINT_BLOCK_OFFSET + SAVE_POINT_BYTES)
        else {
            return false;
        };
        point.copy_from_slice(&self.save_point);
        self.arrays.write_to_block(slot)
    }

    pub fn rank(&self) -> StoryRank {
        let name_end = self
            .phase_name
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(PHASE_NAME_BUFFER_LEN);
        StoryRank::of(&self.phase_name[..name_end], &self.arrays)
    }
}

/// The game's slot checksum: the byte sum of the slot's first 0xC20 bytes.
pub fn slot_checksum(slot: &[u8]) -> Option<u32> {
    let covered = slot.get(..SLOT_CHECKSUM_COVERED_BYTES)?;
    Some(
        covered
            .iter()
            .fold(0_u32, |sum, &byte| sum.wrapping_add(u32::from(byte))),
    )
}

pub fn fix_slot_checksum(slot: &mut [u8]) -> bool {
    let Some(checksum) = slot_checksum(slot) else {
        return false;
    };
    let Some(field) = slot.get_mut(SLOT_CHECKSUM_OFFSET..SLOT_CHECKSUM_OFFSET + 4) else {
        return false;
    };
    field.copy_from_slice(&checksum.to_le_bytes());
    true
}

pub fn slot_player_name(slot: &[u8]) -> Option<&[u8]> {
    slot.get(PLAYER_NAME_BLOCK_OFFSET..PLAYER_NAME_BLOCK_OFFSET + PLAYER_NAME_BYTES)
}

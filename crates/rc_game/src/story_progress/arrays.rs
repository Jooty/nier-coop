use rc_proto::{STORY_ARRAY_BYTES, StoryArray, StoryChange};

use crate::story_progress::{
    GAME_COUNTERS_BLOCK_OFFSET, GAME_FLAGS_BLOCK_OFFSET, KEY_FRAGMENTS_BLOCK_OFFSET,
    SNOW_COUNTERS_BLOCK_OFFSET, SNOW_FLAGS_BLOCK_OFFSET,
};

const FNV_OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

/// The story arrays of the save block: two 0x800-bit flag sets, two 256-byte counter arrays and
/// the five key fragment bytes, indexed in [`StoryArray::ALL`] order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryArrays {
    arrays: [[u8; STORY_ARRAY_BYTES]; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoryKey {
    pub array: StoryArray,
    pub id: u16,
}

impl Default for StoryArrays {
    fn default() -> Self {
        Self {
            arrays: [[0; STORY_ARRAY_BYTES]; 5],
        }
    }
}

impl StoryArrays {
    /// Reads the arrays out of a save block or a save slot, which share one layout.
    pub fn from_block(block: &[u8]) -> Option<Self> {
        let mut arrays = Self::default();
        for array in StoryArray::ALL {
            let start = block_offset(array);
            let used = array.used_bytes();
            let bytes = block.get(start..start + used)?;
            arrays.arrays[index(array)][..used].copy_from_slice(bytes);
        }
        Some(arrays)
    }

    pub fn write_to_block(&self, block: &mut [u8]) -> bool {
        for array in StoryArray::ALL {
            let start = block_offset(array);
            let used = array.used_bytes();
            let Some(target) = block.get_mut(start..start + used) else {
                return false;
            };
            target.copy_from_slice(&self.arrays[index(array)][..used]);
        }
        true
    }

    pub fn bytes(&self, array: StoryArray) -> &[u8] {
        &self.arrays[index(array)][..array.used_bytes()]
    }

    pub fn set_bytes(&mut self, array: StoryArray, bytes: &[u8]) -> bool {
        if bytes.len() != array.used_bytes() {
            return false;
        }
        self.arrays[index(array)][..bytes.len()].copy_from_slice(bytes);
        true
    }

    pub fn flag(&self, array: StoryArray, id: u16) -> bool {
        let id = usize::from(id);
        self.arrays[index(array)]
            .get(id / 8)
            .is_some_and(|byte| byte & (1 << (id % 8)) != 0)
    }

    pub fn counter(&self, array: StoryArray, id: u8) -> u8 {
        self.arrays[index(array)][usize::from(id)]
    }

    pub fn set_flag_count(&self, array: StoryArray) -> u32 {
        self.arrays[index(array)]
            .iter()
            .map(|byte| byte.count_ones())
            .sum()
    }

    pub fn set_flags_among(&self, array: StoryArray, ids: &[u16]) -> u32 {
        let set = ids.iter().filter(|&&id| self.flag(array, id)).count();
        u32::try_from(set).unwrap_or(u32::MAX)
    }

    pub fn apply(&mut self, change: StoryChange) {
        match change {
            StoryChange::Flag { array, id, on } => {
                let id = usize::from(id);
                if let Some(byte) = self.arrays[index(array)].get_mut(id / 8) {
                    let bit = 1 << (id % 8);
                    if on {
                        *byte |= bit;
                    } else {
                        *byte &= !bit;
                    }
                }
            }
            StoryChange::Counter { array, id, value } => {
                self.arrays[index(array)][usize::from(id)] = value;
            }
        }
    }

    /// The changes that turn `self` into `target`: every flag clear first, then every flag set,
    /// then the counters, each in id order (sets run the game's quest rescan, so clears go first).
    pub fn changes_to(&self, target: &Self) -> Vec<StoryChange> {
        let mut clears = Vec::new();
        let mut sets = Vec::new();
        let mut counters = Vec::new();
        for array in StoryArray::ALL {
            let from = self.bytes(array);
            let to = target.bytes(array);
            for (position, (&old, &new)) in from.iter().zip(to.iter()).enumerate() {
                if old == new {
                    continue;
                }
                if array.holds_flags() {
                    for bit in 0..8_u16 {
                        let mask = 1 << bit;
                        if old & mask == new & mask {
                            continue;
                        }
                        let id = u16::try_from(position).unwrap_or(u16::MAX) * 8 + bit;
                        let on = new & mask != 0;
                        let change = StoryChange::Flag { array, id, on };
                        if on {
                            sets.push(change);
                        } else {
                            clears.push(change);
                        }
                    }
                } else {
                    counters.push(StoryChange::Counter {
                        array,
                        id: u8::try_from(position).unwrap_or(u8::MAX),
                        value: new,
                    });
                }
            }
        }
        clears.extend(sets);
        clears.extend(counters);
        clears
    }

    pub fn digest(&self) -> u64 {
        fnv_hash(self.arrays.iter().flatten())
    }
}

impl StoryKey {
    pub fn of(change: StoryChange) -> Self {
        match change {
            StoryChange::Flag { array, id, .. } => Self { array, id },
            StoryChange::Counter { array, id, .. } => Self {
                array,
                id: u16::from(id),
            },
        }
    }
}

pub(crate) fn fnv_hash<'a>(bytes: impl IntoIterator<Item = &'a u8>) -> u64 {
    bytes.into_iter().fold(FNV_OFFSET_BASIS, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

fn index(array: StoryArray) -> usize {
    match array {
        StoryArray::GameFlags => 0,
        StoryArray::SnowFlags => 1,
        StoryArray::GameCounters => 2,
        StoryArray::SnowCounters => 3,
        StoryArray::KeyFragments => 4,
    }
}

fn block_offset(array: StoryArray) -> usize {
    match array {
        StoryArray::GameFlags => GAME_FLAGS_BLOCK_OFFSET,
        StoryArray::SnowFlags => SNOW_FLAGS_BLOCK_OFFSET,
        StoryArray::GameCounters => GAME_COUNTERS_BLOCK_OFFSET,
        StoryArray::SnowCounters => SNOW_COUNTERS_BLOCK_OFFSET,
        StoryArray::KeyFragments => KEY_FRAGMENTS_BLOCK_OFFSET,
    }
}

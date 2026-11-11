use crate::game::{GameError, text_up_to_nul};
use crate::memory::GameMemory;
use crate::offsets::{
    ATTACH_BONE_NAME_READ_LEN, ATTACH_ENTRY_NAME_OFFSET, ATTACH_ENTRY_ROOT_ROTATION_FLAG_OFFSET,
    ATTACH_ENTRY_ROTATION_DEGREES_OFFSET, ATTACH_ENTRY_STRIDE, ATTACH_ENTRY_TARGET_OFFSET,
    ATTACH_ENTRY_TRANSLATION_OFFSET, ATTACH_TABLE_ENTRIES_OFFSET, MAX_ATTACH_ENTRIES_SCANNED,
    PLAYER_ATTACH_TABLE_OFFSETS,
};
use crate::puppet::Affine;

const DEGREES_TO_RADIANS: f32 = core::f32::consts::PI / 180.0;

#[derive(Debug, Clone, PartialEq)]
pub struct AttachPoint {
    pub bone: String,
    pub translation: [f32; 3],
    pub rotation_degrees: [f32; 3],
    pub keeps_root_rotation: bool,
    pub target: u8,
}

impl AttachPoint {
    /// The attach frame on an actor whose `bone` and root matrices are given, as the player's
    /// own attach lookup builds it (bone, optionally root rotation, then offset and turn).
    pub fn frame(&self, bone: &Affine, root: &Affine) -> Affine {
        let base = if self.keeps_root_rotation {
            bone.with_rotation_of(root)
        } else {
            *bone
        };
        let mut local = Affine::rotation_xyz(
            self.rotation_degrees
                .map(|angle| angle * DEGREES_TO_RADIANS),
        );
        for (row, value) in local.rows.iter_mut().zip(self.translation) {
            row[3] = value;
        }
        base.then(&local)
    }
}

/// Looks `id` up in the player's attach tables (the per-player list first, then the shared one),
/// as the player's attach lookup does. `None` when no table holds it.
pub fn read_attach_point<M: GameMemory>(
    memory: &M,
    player: u64,
    id: i32,
) -> Result<Option<AttachPoint>, GameError> {
    for table_offset in PLAYER_ATTACH_TABLE_OFFSETS {
        let table = memory.read_u64(player + table_offset)?;
        if table == 0 {
            continue;
        }
        let count = memory.read_i32(table)?.clamp(0, MAX_ATTACH_ENTRIES_SCANNED);
        let relative = memory.read_i32(table + ATTACH_TABLE_ENTRIES_OFFSET)?;
        if relative == 0 {
            continue;
        }
        let entries = table
            .wrapping_add(ATTACH_TABLE_ENTRIES_OFFSET)
            .wrapping_add_signed(i64::from(relative));
        for number in 0..count.cast_unsigned() {
            let entry = entries + u64::from(number) * ATTACH_ENTRY_STRIDE;
            if memory.read_i32(entry)? == id {
                return read_entry(memory, entry).map(Some);
            }
        }
    }
    Ok(None)
}

fn read_entry<M: GameMemory>(memory: &M, entry: u64) -> Result<AttachPoint, GameError> {
    let name_relative = memory.read_i32(entry + ATTACH_ENTRY_NAME_OFFSET)?;
    let bone = if name_relative == 0 {
        String::new()
    } else {
        let name = (entry + ATTACH_ENTRY_NAME_OFFSET).wrapping_add_signed(i64::from(name_relative));
        text_up_to_nul(&memory.read_array::<ATTACH_BONE_NAME_READ_LEN>(name)?)
    };
    let floats = |offset: u64| -> Result<[f32; 3], GameError> {
        Ok([
            memory.read_f32(entry + offset)?,
            memory.read_f32(entry + offset + 4)?,
            memory.read_f32(entry + offset + 8)?,
        ])
    };
    Ok(AttachPoint {
        bone,
        translation: floats(ATTACH_ENTRY_TRANSLATION_OFFSET)?,
        rotation_degrees: floats(ATTACH_ENTRY_ROTATION_DEGREES_OFFSET)?,
        keeps_root_rotation: memory.read_u8(entry + ATTACH_ENTRY_ROOT_ROTATION_FLAG_OFFSET)? != 0,
        target: memory.read_u8(entry + ATTACH_ENTRY_TARGET_OFFSET)?,
    })
}

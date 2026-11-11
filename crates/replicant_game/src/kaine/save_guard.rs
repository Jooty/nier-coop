use replicant_proto::PartnerBody;

use crate::kaine::{
    KAINE_COPY_SAVE_END, KAINE_COPY_SAVE_START, KAINE_PLAYER_FLAG_SAVE_OFFSET,
    KAINE_PLAYER_FLAG_SET, SEED_INDEX_SAVE_OFFSET,
};
use crate::puppet::body_info;
use crate::story_progress::{SAVE_FILE_BYTES, SAVE_FILE_SLOT_COUNT, SLOT_BYTES, SLOTS_FILE_OFFSET};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KaineCopyPutBack {
    pub slot: usize,
    pub from_disk: bool,
    pub seed_index_reads_kaine: bool,
}

/// Puts the save's Kaine copy (flag, HP, level, exp, items, magic, ability slots) back to the
/// file on disk in every slot this write changes that has her flag set; a slot with no valid
/// counterpart on disk gets it zeroed, as a save that never played her has it.
pub fn put_back_kaine_copies(file: &mut [u8], on_disk: Option<&[u8]>) -> Vec<KaineCopyPutBack> {
    if file.len() != SAVE_FILE_BYTES {
        return Vec::new();
    }
    let on_disk = on_disk.filter(|disk| disk.len() == SAVE_FILE_BYTES);
    let kaine_index = body_info(PartnerBody::SeedKaineE).own_seed_table_index;
    let mut put_back = Vec::new();
    for slot in 0..SAVE_FILE_SLOT_COUNT {
        let start = SLOTS_FILE_OFFSET + slot * SLOT_BYTES;
        let slot_range = start..start + SLOT_BYTES;
        let disk_slot = on_disk.map(|disk| &disk[slot_range.clone()]);
        let written = &file[slot_range.clone()];
        if disk_slot == Some(written)
            || read_i32(written, KAINE_PLAYER_FLAG_SAVE_OFFSET) != Some(KAINE_PLAYER_FLAG_SET)
        {
            continue;
        }
        let copy = start + KAINE_COPY_SAVE_START..start + KAINE_COPY_SAVE_END;
        let seed_index_reads_kaine = read_i32(written, SEED_INDEX_SAVE_OFFSET)
            .is_some_and(|index| u32::try_from(index) == Ok(kaine_index));
        match disk_slot {
            Some(disk) => {
                file[copy].copy_from_slice(&disk[KAINE_COPY_SAVE_START..KAINE_COPY_SAVE_END])
            }
            None => file[copy].fill(0),
        }
        put_back.push(KaineCopyPutBack {
            slot,
            from_disk: disk_slot.is_some(),
            seed_index_reads_kaine,
        });
    }
    put_back
}

fn read_i32(slot: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        slot.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

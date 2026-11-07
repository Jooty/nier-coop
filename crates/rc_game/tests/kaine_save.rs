use rc_game::kaine::{
    KAINE_COPY_SAVE_END, KAINE_COPY_SAVE_START, KAINE_PLAYER_FLAG_SAVE_OFFSET,
    SEED_INDEX_SAVE_OFFSET, put_back_kaine_copies,
};
use rc_game::story_progress::{SAVE_FILE_BYTES, SLOT_BYTES, SLOTS_FILE_OFFSET};

const HER_LEVEL_OFFSET: usize = 0xE3C;
const HER_FIRST_ITEM_OFFSET: usize = 0xE44;
const HER_LAST_MAGIC_OFFSET: usize = 0x1160;
const PROTAGONIST_ITEM_OFFSET: usize = 0xC0;

fn slot_start(slot: usize) -> usize {
    SLOTS_FILE_OFFSET + slot * SLOT_BYTES
}

fn put_i32(file: &mut [u8], at: usize, value: i32) {
    file[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn played_as_her(file: &mut [u8], slot: usize) {
    let start = slot_start(slot);
    put_i32(file, start + KAINE_PLAYER_FLAG_SAVE_OFFSET, 1);
    put_i32(file, start + HER_LEVEL_OFFSET, 7);
    file[start + HER_FIRST_ITEM_OFFSET] = 3;
    file[start + HER_LAST_MAGIC_OFFSET] = 1;
}

#[test]
fn her_copy_goes_back_to_what_the_disk_holds_and_the_rest_of_the_write_stays() {
    let mut disk = vec![0_u8; SAVE_FILE_BYTES];
    disk[slot_start(2) + HER_FIRST_ITEM_OFFSET] = 9;
    let mut file = disk.clone();
    played_as_her(&mut file, 2);
    file[slot_start(2) + PROTAGONIST_ITEM_OFFSET] = 5;
    let put_back = put_back_kaine_copies(&mut file, Some(&disk));
    assert_eq!(put_back.len(), 1);
    assert_eq!(put_back[0].slot, 2);
    assert!(put_back[0].from_disk);
    let start = slot_start(2);
    assert_eq!(
        file[start + KAINE_COPY_SAVE_START..start + KAINE_COPY_SAVE_END],
        disk[start + KAINE_COPY_SAVE_START..start + KAINE_COPY_SAVE_END]
    );
    assert_eq!(file[start + PROTAGONIST_ITEM_OFFSET], 5);
}

#[test]
fn with_no_file_on_disk_her_copy_is_zeroed() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    played_as_her(&mut file, 0);
    let put_back = put_back_kaine_copies(&mut file, None);
    assert_eq!(put_back.len(), 1);
    assert!(!put_back[0].from_disk);
    let start = slot_start(0);
    assert!(
        file[start + KAINE_COPY_SAVE_START..start + KAINE_COPY_SAVE_END]
            .iter()
            .all(|&byte| byte == 0)
    );
}

#[test]
fn a_slot_this_write_does_not_change_is_left_alone_even_with_her_flag() {
    let mut disk = vec![0_u8; SAVE_FILE_BYTES];
    played_as_her(&mut disk, 4);
    let mut file = disk.clone();
    assert!(put_back_kaine_copies(&mut file, Some(&disk)).is_empty());
    assert_eq!(file, disk);
}

#[test]
fn a_changed_slot_without_her_flag_is_left_alone() {
    let disk = vec![0_u8; SAVE_FILE_BYTES];
    let mut file = disk.clone();
    file[slot_start(1) + HER_LEVEL_OFFSET] = 4;
    let before = file.clone();
    assert!(put_back_kaine_copies(&mut file, Some(&disk)).is_empty());
    assert_eq!(file, before);
}

#[test]
fn a_seed_index_of_four_is_reported_and_never_rewritten() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    played_as_her(&mut file, 3);
    put_i32(&mut file, slot_start(3) + SEED_INDEX_SAVE_OFFSET, 4);
    let put_back = put_back_kaine_copies(&mut file, None);
    assert!(put_back[0].seed_index_reads_kaine);
    let at = slot_start(3) + SEED_INDEX_SAVE_OFFSET;
    assert_eq!(file[at..at + 4], 4_i32.to_le_bytes());
}

#[test]
fn a_buffer_that_is_not_a_whole_save_file_is_never_touched() {
    let mut short = vec![1_u8; SAVE_FILE_BYTES - 1];
    assert!(put_back_kaine_copies(&mut short, None).is_empty());
    assert!(short.iter().all(|&byte| byte == 1));
}

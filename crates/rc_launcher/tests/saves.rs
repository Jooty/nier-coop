use chrono::{TimeZone, Utc};
use rc_launcher::saves::{BackupError, backup_saves, format_stamp};

use crate::scratch::fresh_dir;

#[test]
fn backup_copies_every_file_into_a_stamped_folder() {
    let base = fresh_dir("backup_copy");
    let saves = base.join("saves");
    std::fs::create_dir_all(saves.join("slot")).unwrap();
    std::fs::write(saves.join("a.dat"), b"a").unwrap();
    std::fs::write(saves.join("slot/b.dat"), b"b").unwrap();
    let (destination, files) =
        backup_saves(&saves, &base.join("backups"), "20261106-120000").unwrap();
    assert_eq!(files, 2);
    assert_eq!(std::fs::read(destination.join("slot/b.dat")).unwrap(), b"b");
}

#[test]
fn backup_refuses_to_overwrite_and_reports_a_missing_save_folder() {
    let base = fresh_dir("backup_refuse");
    let saves = base.join("saves");
    std::fs::create_dir_all(&saves).unwrap();
    backup_saves(&saves, &base.join("backups"), "s").unwrap();
    assert!(matches!(
        backup_saves(&saves, &base.join("backups"), "s"),
        Err(BackupError::AlreadyExists(_))
    ));
    assert!(matches!(
        backup_saves(&base.join("nope"), &base.join("backups"), "t"),
        Err(BackupError::NoSaveFolder(_))
    ));
}

#[test]
fn stamp_has_the_same_shape_as_the_shell_script() {
    let moment = Utc.with_ymd_and_hms(2026, 11, 6, 9, 5, 3).unwrap();
    assert_eq!(format_stamp(&moment), "20261106-090503");
}

fn gamedata_with_slot(slot: usize, name: &str, phase: &str, level: i32, seconds: f64) -> Vec<u8> {
    let mut bytes = vec![0u8; rc_launcher::saves::GAMEDATA_SIZE];
    for index in 0..rc_launcher::saves::SLOT_COUNT as usize {
        let start = rc_launcher::saves::SLOTS_START + index * rc_launcher::saves::SLOT_STRIDE;
        bytes[start..start + 4].copy_from_slice(&200u32.to_le_bytes());
    }
    let start = rc_launcher::saves::SLOTS_START + slot * rc_launcher::saves::SLOT_STRIDE;
    bytes[start + 4..start + 4 + phase.len()].copy_from_slice(phase.as_bytes());
    bytes[start + 0x2C..start + 0x2C + name.len()].copy_from_slice(name.as_bytes());
    bytes[start + 0x64..start + 0x68].copy_from_slice(&level.to_le_bytes());
    bytes[start + 0x4A0..start + 0x4A8].copy_from_slice(&seconds.to_le_bytes());
    bytes
}

#[test]
fn only_a_slot_with_a_phase_counts_as_used() {
    let bytes = gamedata_with_slot(2, "Jay", "A_CENTER_VILLAGE_01", 12, 3725.0);
    let slots = rc_launcher::saves::parse_slots(&bytes).unwrap();
    assert_eq!(slots.len(), 7);
    assert_eq!(slots.iter().filter(|slot| slot.used).count(), 1);
    assert_eq!(slots[2].name, "Jay");
    assert_eq!(slots[2].level, 12);
    assert_eq!(slots[2].play_time_text(), "1:02:05");
}

#[test]
fn the_default_slot_is_the_used_one_with_most_play_time() {
    let mut bytes = gamedata_with_slot(1, "A", "P", 1, 100.0);
    let other = gamedata_with_slot(4, "B", "P", 1, 900.0);
    let start = rc_launcher::saves::SLOTS_START + 4 * rc_launcher::saves::SLOT_STRIDE;
    let end = start + rc_launcher::saves::SLOT_STRIDE;
    bytes[start..end].copy_from_slice(&other[start..end]);
    let slots = rc_launcher::saves::parse_slots(&bytes).unwrap();
    assert_eq!(rc_launcher::saves::default_slot(&slots), Some(4));
    let empty =
        rc_launcher::saves::parse_slots(&vec![0u8; rc_launcher::saves::GAMEDATA_SIZE]).unwrap();
    assert_eq!(rc_launcher::saves::default_slot(&empty), None);
}

#[test]
fn a_file_of_the_wrong_size_is_refused() {
    assert!(matches!(
        rc_launcher::saves::parse_slots(&[0u8; 10]),
        Err(rc_launcher::saves::SlotError::WrongSize(10))
    ));
}

#[test]
fn restoring_a_backup_first_keeps_the_current_saves() {
    let base = fresh_dir("restore_keeps");
    let saves = base.join("saves");
    let backups = base.join("backups");
    std::fs::create_dir_all(&saves).unwrap();
    std::fs::write(saves.join("GAMEDATA"), b"old").unwrap();
    backup_saves(&saves, &backups, "20261106-100000").unwrap();
    std::fs::write(saves.join("GAMEDATA"), b"new").unwrap();
    rc_launcher::saves::restore_backup(&saves, &backups, "20261106-100000", "20261106-110000")
        .unwrap();
    assert_eq!(std::fs::read(saves.join("GAMEDATA")).unwrap(), b"old");
    let kept = backups.join("before-restore-20261106-110000/GAMEDATA");
    assert_eq!(std::fs::read(kept).unwrap(), b"new");
    assert_eq!(
        rc_launcher::saves::list_backups(&backups)[0],
        "before-restore-20261106-110000"
    );
}

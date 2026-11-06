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
fn the_player_name_is_the_name_in_the_booted_slot() {
    let bytes = gamedata_with_slot(2, "Jay", "A_CENTER_VILLAGE_01", 12, 3725.0);
    let slots = rc_launcher::saves::parse_slots(&bytes).unwrap();
    assert_eq!(
        rc_launcher::saves::slot_player_name(&slots, Some(2)).as_deref(),
        Some("Jay")
    );
}

#[test]
fn an_unused_or_unselected_slot_gives_no_player_name() {
    let bytes = gamedata_with_slot(2, "Jay", "A_CENTER_VILLAGE_01", 12, 3725.0);
    let slots = rc_launcher::saves::parse_slots(&bytes).unwrap();
    assert_eq!(rc_launcher::saves::slot_player_name(&slots, Some(3)), None);
    assert_eq!(rc_launcher::saves::slot_player_name(&slots, None), None);
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

fn save_folder_with_normal_slot(name: &str, slot: usize) -> std::path::PathBuf {
    let saves = fresh_dir(name);
    let user = saves.join("Steam").join("76561198000000000");
    std::fs::create_dir_all(&user).unwrap();
    let bytes = gamedata_with_slot(slot, "Jay", "A_CENTER_VILLAGE_01", 12, 3725.0);
    std::fs::write(user.join("GAMEDATA"), bytes).unwrap();
    saves
}

#[test]
fn copying_a_slot_creates_the_coop_file_and_leaves_the_normal_file_alone() {
    let saves = save_folder_with_normal_slot("coop_copy", 2);
    let normal = saves.join("Steam/76561198000000000/GAMEDATA");
    let before = std::fs::read(&normal).unwrap();
    let coop = rc_launcher::saves::copy_slot_into_coop(&saves, 2, 0).unwrap();
    assert!(coop.ends_with("GAMEDATA.coop"));
    assert_eq!(std::fs::read(&normal).unwrap(), before);
    let slots = rc_launcher::saves::read_coop_slots(&saves).unwrap();
    assert_eq!(slots.iter().filter(|slot| slot.used).count(), 1);
    assert_eq!(slots[0].name, "Jay");
    assert_eq!(rc_launcher::saves::first_empty_slot(&slots), Some(1));
}

#[test]
fn a_missing_coop_file_reads_as_not_found() {
    let saves = save_folder_with_normal_slot("coop_missing", 0);
    assert!(matches!(
        rc_launcher::saves::read_coop_slots(&saves),
        Err(rc_launcher::saves::SlotError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound
    ));
}

#[test]
fn coop_backup_and_restore_round_trip_with_a_safety_copy() {
    let saves = save_folder_with_normal_slot("coop_backup", 0);
    let backups = fresh_dir("coop_backup_root");
    rc_launcher::saves::copy_slot_into_coop(&saves, 0, 0).unwrap();
    let coop = saves.join("Steam/76561198000000000/GAMEDATA.coop");
    let original = std::fs::read(&coop).unwrap();
    rc_launcher::saves::backup_coop_file(&saves, &backups, "one").unwrap();
    std::fs::write(&coop, b"changed").unwrap();
    rc_launcher::saves::restore_coop_backup(&saves, &backups, "one", "now").unwrap();
    assert_eq!(std::fs::read(&coop).unwrap(), original);
    assert!(backups.join("before-restore-now/GAMEDATA.coop").is_file());
}

#[test]
fn an_unreadable_coop_file_is_never_copied_over() {
    let saves = save_folder_with_normal_slot("coop_target_bad", 0);
    let coop = saves.join("Steam/76561198000000000/GAMEDATA.coop");
    std::fs::write(&coop, b"short").unwrap();
    assert!(matches!(
        rc_launcher::saves::coop_copy_target_slot(&saves),
        Err(rc_launcher::saves::CoopTargetError::Unreadable(_))
    ));
}

#[test]
fn the_copy_target_is_slot_zero_only_when_there_is_no_coop_file() {
    let saves = save_folder_with_normal_slot("coop_target_new", 0);
    assert_eq!(
        rc_launcher::saves::coop_copy_target_slot(&saves).unwrap(),
        0
    );
    rc_launcher::saves::copy_slot_into_coop(&saves, 0, 0).unwrap();
    assert_eq!(
        rc_launcher::saves::coop_copy_target_slot(&saves).unwrap(),
        1
    );
}

#[test]
fn restoring_a_wrong_sized_backup_is_refused_and_changes_nothing() {
    let saves = save_folder_with_normal_slot("coop_restore_bad", 0);
    let backups = fresh_dir("coop_restore_bad_root");
    rc_launcher::saves::copy_slot_into_coop(&saves, 0, 0).unwrap();
    let coop = saves.join("Steam/76561198000000000/GAMEDATA.coop");
    let original = std::fs::read(&coop).unwrap();
    std::fs::create_dir_all(backups.join("bad")).unwrap();
    std::fs::write(backups.join("bad/GAMEDATA.coop"), b"tiny").unwrap();
    assert!(matches!(
        rc_launcher::saves::restore_coop_backup(&saves, &backups, "bad", "now"),
        Err(rc_launcher::saves::BackupError::WrongSize(4))
    ));
    assert_eq!(std::fs::read(&coop).unwrap(), original);
}

#[test]
fn two_steam_folders_without_a_game_file_are_not_guessed() {
    let saves = fresh_dir("coop_two_folders");
    std::fs::create_dir_all(saves.join("Steam/1")).unwrap();
    std::fs::create_dir_all(saves.join("Steam/2")).unwrap();
    assert_eq!(rc_launcher::saves::coop_gamedata_path(&saves), None);
}

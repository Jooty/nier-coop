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

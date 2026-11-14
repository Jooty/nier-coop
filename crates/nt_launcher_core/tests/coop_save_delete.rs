use std::path::{Path, PathBuf};

use nt_launcher_core::{
    CoopSaveDeleteError, CoopSaveDeleteOffer, DELETED_SAVES_FOLDERS_KEPT, coop_save_delete_offer,
    coop_saves_in, coop_saves_in_whole_folder, delete_coop_saves, is_coop_save_name,
    whole_coop_saves_folder,
};

const STEAM_ID_FOLDER: &str = "Steam/76561190000000000";
const COOP_SAVE_FILES: [&str; 5] = [
    "GAMEDATA.coop",
    "GAMEDATA.coop.bak",
    "GAMEDATA.coop.bodies.toml",
    "GAMEDATA.coop.campaigns.toml",
    "GAMEDATA.seat-0a1b2c3d",
];
const COOP_SEATS_FOLDER: &str = "GAMEDATA.coop.seats";
const SEAT_FILES: [&str; 2] = ["0a1b2c3d.seat", "0a1b2c3d.bak"];
const OTHER_FILES: [&str; 4] = [
    "GAMEDATA",
    "GAMEDATA.bak",
    "steam_autocloud.vdf",
    "notes.txt",
];
const STAMP: &str = "20261113-120000";

struct SaveFolder {
    save_dir: PathBuf,
    folder: PathBuf,
    backups_root: PathBuf,
}

fn scratch_root(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("test-scratch/coop-save-delete")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder");
    dir
}

fn contents_of(name: &str) -> Vec<u8> {
    format!("contents of {name}").into_bytes()
}

fn save_folder_with_both(name: &str) -> SaveFolder {
    let root = scratch_root(name);
    let save_dir = root.join("My Games/game");
    let folder = save_dir.join(STEAM_ID_FOLDER);
    let seats = folder.join(COOP_SEATS_FOLDER);
    std::fs::create_dir_all(&seats).expect("seats folder");
    for file in COOP_SAVE_FILES.iter().chain(OTHER_FILES.iter()) {
        std::fs::write(folder.join(file), contents_of(file)).expect("save file");
    }
    for file in SEAT_FILES {
        std::fs::write(seats.join(file), contents_of(file)).expect("seat file");
    }
    SaveFolder {
        save_dir,
        folder,
        backups_root: root.join("launcher/save-backups"),
    }
}

fn found_in(saves: &SaveFolder) -> Vec<PathBuf> {
    coop_saves_in(std::slice::from_ref(&saves.folder))
}

fn deleted_folders_in(backups_root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(backups_root)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                .filter(|name| name.ends_with("-deleted"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn assert_every_save_still_there(saves: &SaveFolder) {
    for file in COOP_SAVE_FILES.iter().chain(OTHER_FILES.iter()) {
        assert_eq!(
            std::fs::read(saves.folder.join(file)).expect("still there"),
            contents_of(file)
        );
    }
    for file in SEAT_FILES {
        assert!(saves.folder.join(COOP_SEATS_FOLDER).join(file).is_file());
    }
}

#[test]
fn only_the_co_op_saves_are_chosen_and_never_the_games_own_save() {
    let saves = save_folder_with_both("choice");
    let mut expected: Vec<PathBuf> = COOP_SAVE_FILES
        .iter()
        .chain([COOP_SEATS_FOLDER].iter())
        .map(|name| saves.folder.join(name))
        .collect();
    expected.sort();
    assert_eq!(found_in(&saves), expected);
    assert!(!is_coop_save_name("GAMEDATA"));
    assert!(!is_coop_save_name("GAMEDATA.bak"));
    assert!(is_coop_save_name("GAMEDATA.seat-ffffffff"));
}

#[test]
fn a_folder_that_cannot_be_read_holds_no_co_op_saves() {
    let root = scratch_root("missing");
    assert!(coop_saves_in(&[root.join("not-there")]).is_empty());
}

#[test]
fn a_delete_moves_the_co_op_saves_aside_and_leaves_every_other_file() {
    let saves = save_folder_with_both("delete");
    let found = found_in(&saves);
    let deleted =
        delete_coop_saves(&saves.save_dir, &found, &saves.backups_root, STAMP).expect("deleted");
    assert_eq!(
        deleted.folder,
        saves.backups_root.join("20261113-120000-deleted")
    );
    assert_eq!(deleted.files, COOP_SAVE_FILES.len() + SEAT_FILES.len());
    assert!(found_in(&saves).is_empty());
    for file in OTHER_FILES {
        assert_eq!(
            std::fs::read(saves.folder.join(file)).expect("kept"),
            contents_of(file)
        );
    }
    let moved = deleted.folder.join(STEAM_ID_FOLDER);
    for file in COOP_SAVE_FILES {
        assert_eq!(
            std::fs::read(moved.join(file)).expect("moved"),
            contents_of(file)
        );
    }
    for file in SEAT_FILES {
        assert_eq!(
            std::fs::read(moved.join(COOP_SEATS_FOLDER).join(file)).expect("moved seat"),
            contents_of(file)
        );
    }
    assert!(!moved.join("GAMEDATA").exists());
}

#[test]
fn deleting_is_refused_while_the_game_runs_or_its_lobby_is_open() {
    assert_eq!(
        coop_save_delete_offer(3, true, false),
        CoopSaveDeleteOffer::GameRunning
    );
    assert_eq!(
        coop_save_delete_offer(3, false, true),
        CoopSaveDeleteOffer::LobbyOpen
    );
    assert_eq!(
        coop_save_delete_offer(0, false, false),
        CoopSaveDeleteOffer::NothingToDelete
    );
    assert!(coop_save_delete_offer(3, false, false).allowed());
    assert!(!coop_save_delete_offer(3, true, true).allowed());
}

#[test]
fn nothing_to_delete_writes_no_folder() {
    let root = scratch_root("nothing");
    let backups_root = root.join("save-backups");
    let refused = delete_coop_saves(&root, &[], &backups_root, STAMP);
    assert!(matches!(refused, Err(CoopSaveDeleteError::NothingToDelete)));
    assert!(!backups_root.exists());
}

#[test]
fn only_the_newest_three_deleted_folders_are_kept() {
    let saves = save_folder_with_both("keep-three");
    let older = [
        "20261101-080000-deleted",
        "20261102-080000-deleted",
        "20261103-080000-deleted",
        "20261104-080000-deleted",
    ];
    for name in older {
        std::fs::create_dir_all(saves.backups_root.join(name)).expect("older folder");
    }
    let plain_backup = saves.backups_root.join("20261001-080000");
    std::fs::create_dir_all(&plain_backup).expect("plain backup");
    delete_coop_saves(
        &saves.save_dir,
        &found_in(&saves),
        &saves.backups_root,
        STAMP,
    )
    .expect("deleted");
    assert_eq!(DELETED_SAVES_FOLDERS_KEPT, 3);
    assert_eq!(
        deleted_folders_in(&saves.backups_root),
        [
            "20261103-080000-deleted",
            "20261104-080000-deleted",
            "20261113-120000-deleted"
        ]
    );
    assert!(plain_backup.is_dir());
}

#[test]
fn a_folder_already_there_for_the_stamp_is_not_written_over() {
    let saves = save_folder_with_both("already-there");
    let taken = saves.backups_root.join("20261113-120000-deleted");
    std::fs::create_dir_all(&taken).expect("taken folder");
    let refused = delete_coop_saves(
        &saves.save_dir,
        &found_in(&saves),
        &saves.backups_root,
        STAMP,
    );
    assert!(matches!(refused, Err(CoopSaveDeleteError::AlreadyExists(path)) if path == taken));
    assert_every_save_still_there(&saves);
}

#[cfg(unix)]
#[test]
fn a_copy_that_fails_part_way_deletes_nothing_and_leaves_no_half_folder() {
    let saves = save_folder_with_both("half-failed");
    let seats = saves.folder.join(COOP_SEATS_FOLDER);
    std::os::unix::fs::symlink(seats.join("gone.seat"), seats.join("broken.seat"))
        .expect("dangling link");
    let failed = delete_coop_saves(
        &saves.save_dir,
        &found_in(&saves),
        &saves.backups_root,
        STAMP,
    );
    assert!(matches!(failed, Err(CoopSaveDeleteError::CopyFailed(_))));
    assert_every_save_still_there(&saves);
    assert!(deleted_folders_in(&saves.backups_root).is_empty());
}

#[test]
fn a_game_with_a_folder_of_only_co_op_saves_is_found_by_its_id() {
    assert_eq!(
        whole_coop_saves_folder("automata"),
        Some("NieR_Automata_coop")
    );
    assert_eq!(whole_coop_saves_folder("replicant"), None);
}

#[test]
fn a_whole_co_op_folder_is_moved_aside_with_its_host_and_guest_folders() {
    let root = scratch_root("whole-folder");
    let my_games = root.join("My Games");
    let coop = my_games.join("NieR_Automata_coop");
    for role in ["host", "guest"] {
        std::fs::create_dir_all(coop.join(role)).expect("role folder");
        std::fs::write(coop.join(role).join("SlotData_0.dat"), contents_of(role)).expect("slot");
    }
    let normal = my_games.join("NieR_Automata/SlotData_0.dat");
    std::fs::create_dir_all(normal.parent().expect("parent")).expect("normal folder");
    std::fs::write(&normal, b"own save").expect("own save");
    let found = coop_saves_in_whole_folder(&coop);
    assert_eq!(found, vec![coop.join("guest"), coop.join("host")]);
    let backups = root.join("launcher/save-backups");
    let deleted = delete_coop_saves(&coop, &found, &backups, STAMP).expect("deleted");
    assert_eq!(deleted.files, 2);
    assert!(coop_saves_in_whole_folder(&coop).is_empty());
    assert_eq!(std::fs::read(&normal).expect("own save kept"), b"own save");
    assert_eq!(
        std::fs::read(deleted.folder.join("host/SlotData_0.dat")).expect("copy"),
        contents_of("host")
    );
}

#[test]
fn a_whole_co_op_folder_that_cannot_be_read_holds_no_co_op_saves() {
    let root = scratch_root("whole-missing");
    assert!(coop_saves_in_whole_folder(&root.join("not-there")).is_empty());
}

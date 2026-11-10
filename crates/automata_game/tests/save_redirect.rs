use automata_game::save_redirect::{
    SAVE_PATH_BUFFER_LEN, SaveFileKind, SaveRedirect, SaveRedirectStatus, SaveRole,
    redirect_save_path,
};

const NORMAL_SLOT: &str =
    "C:\\Users\\steamuser\\Documents\\My Games\\NieR_Automata\\SlotData_2.dat";

fn redirected(path: &str, role: SaveRole) -> (String, String) {
    match redirect_save_path(path, role) {
        SaveRedirect::Redirected { path, folder } => (path, folder),
        other => panic!("expected a redirect, got {other:?}"),
    }
}

#[test]
fn each_file_kind_moves_into_the_role_folder() {
    let base = "C:\\Users\\steamuser\\Documents\\My Games\\";
    for (file, role, folder_name) in [
        ("SlotData_0.dat", SaveRole::Host, "host"),
        ("SlotData_3.dat", SaveRole::Guest, "guest"),
        ("GameData.dat", SaveRole::Host, "host"),
        ("GameData.dat", SaveRole::Guest, "guest"),
        ("SystemData.dat", SaveRole::Host, "host"),
        ("SystemData.dat", SaveRole::Guest, "guest"),
    ] {
        let normal = format!("{base}NieR_Automata\\{file}");
        let (path, folder) = redirected(&normal, role);
        assert_eq!(folder, format!("{base}NieR_Automata_coop\\{folder_name}"));
        assert_eq!(path, format!("{folder}\\{file}"));
    }
}

#[test]
fn host_and_guest_never_share_a_path() {
    let (host, _) = redirected(NORMAL_SLOT, SaveRole::Host);
    let (guest, _) = redirected(NORMAL_SLOT, SaveRole::Guest);
    assert_ne!(host, guest);
    assert_ne!(host, NORMAL_SLOT);
}

#[test]
fn a_redirected_path_is_left_alone() {
    let (path, _) = redirected(NORMAL_SLOT, SaveRole::Host);
    assert_eq!(
        redirect_save_path(&path, SaveRole::Host),
        SaveRedirect::AlreadyRedirected
    );
    assert_eq!(
        redirect_save_path(&path, SaveRole::Guest),
        SaveRedirect::AlreadyRedirected
    );
}

#[test]
fn the_folder_match_ignores_case() {
    let lower = "c:\\users\\x\\documents\\my games\\nier_automata\\GameData.dat";
    let (path, _) = redirected(lower, SaveRole::Guest);
    assert_eq!(
        path,
        "c:\\users\\x\\documents\\My Games\\NieR_Automata_coop\\guest\\GameData.dat"
    );
}

#[test]
fn paths_that_are_not_save_files_are_not_touched() {
    for path in [
        "C:\\Users\\x\\AppData\\Roaming\\PlatinumGames\\NierAutomata\\config.ini",
        "C:\\Users\\x\\Documents\\My Games\\NieR_Automata\\config.ini",
        "C:\\Users\\x\\Documents\\My Games\\NieR_Automata\\SlotData_.dat",
        "C:\\Users\\x\\Documents\\My Games\\NieR_Automata\\SlotData_a.dat",
        "C:\\Users\\x\\Documents\\My Games\\NieR_Automata\\sub\\GameData.dat",
        "C:\\Users\\x\\Documents\\My Games\\OtherGame\\GameData.dat",
        "",
    ] {
        assert_eq!(
            redirect_save_path(path, SaveRole::Host),
            SaveRedirect::NotASavePath,
            "{path}"
        );
    }
}

#[test]
fn a_result_that_cannot_fit_the_games_buffer_is_refused() {
    let long_user = "u".repeat(SAVE_PATH_BUFFER_LEN - 45);
    let path = format!("C:\\{long_user}\\My Games\\NieR_Automata\\SystemData.dat");
    assert!(path.len() < SAVE_PATH_BUFFER_LEN);
    assert_eq!(
        redirect_save_path(&path, SaveRole::Guest),
        SaveRedirect::TooLong
    );
}

#[test]
fn file_kinds_follow_the_games_three_format_strings() {
    assert_eq!(
        SaveFileKind::of_file_name("SlotData_12.dat"),
        Some(SaveFileKind::Slot(12))
    );
    assert_eq!(
        SaveFileKind::of_file_name("GameData.dat"),
        Some(SaveFileKind::Game)
    );
    assert_eq!(
        SaveFileKind::of_file_name("SystemData.dat"),
        Some(SaveFileKind::System)
    );
    assert_eq!(SaveFileKind::of_file_name("SlotData_-1.dat"), None);
    assert_eq!(SaveFileKind::of_file_name("slotdata_1.dat"), None);
}

#[test]
fn the_status_lines_are_the_ones_the_menu_shows() {
    assert_eq!(
        SaveRedirectStatus::CoopFolder.status_line(),
        "saves: co-op folder"
    );
    assert_eq!(
        SaveRedirectStatus::NotRedirected.status_line(),
        "saves: NOT redirected, the normal save folder is in use"
    );
}

use rc_game::save_files::{
    COOP_GAMEDATA_FILE_NAME, ReloadProgress, coop_save_name, reload_progress,
};

#[test]
fn the_game_save_file_is_redirected_to_the_coop_file() {
    let redirected = coop_save_name(b"GAMEDATA").expect("GAMEDATA is redirected");
    assert_eq!(redirected, b"GAMEDATA.coop\0");
    assert_eq!(
        &redirected[..redirected.len() - 1],
        COOP_GAMEDATA_FILE_NAME.as_bytes()
    );
}

#[test]
fn other_files_and_the_coop_file_itself_are_left_alone() {
    assert_eq!(coop_save_name(b"drawing_settings.ini"), None);
    assert_eq!(coop_save_name(b"GAMEDATA.coop"), None);
    assert_eq!(coop_save_name(b"gamedata"), None);
}

#[test]
fn an_idle_manager_is_asked_to_reload_once() {
    assert_eq!(reload_progress(false, 0, 0), ReloadProgress::Request);
}

#[test]
fn a_busy_manager_is_waited_for_before_and_after_the_request() {
    assert_eq!(reload_progress(false, 2, 0), ReloadProgress::Wait);
    assert_eq!(reload_progress(true, 1, 0), ReloadProgress::Wait);
    assert_eq!(reload_progress(true, 2, 0), ReloadProgress::Wait);
}

#[test]
fn a_finished_reload_is_loaded_when_the_result_is_zero_and_failed_otherwise() {
    assert_eq!(reload_progress(true, 0, 0), ReloadProgress::Loaded);
    assert_eq!(reload_progress(true, 0, 0x11), ReloadProgress::Failed(0x11));
}

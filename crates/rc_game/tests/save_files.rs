use rc_game::save_files::{COOP_GAMEDATA_FILE_NAME, coop_save_name};

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

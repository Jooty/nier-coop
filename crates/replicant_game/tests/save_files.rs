use replicant_game::save_files::{
    COOP_GAMEDATA_FILE_NAME, RELOAD_BUSY_GRACE_MS, RELOAD_TIMEOUT_MS, ReloadInputs, ReloadProgress,
    coop_save_name, reload_progress,
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

fn inputs(requested: bool, state: i32, result: u8) -> ReloadInputs {
    ReloadInputs {
        requested,
        busy_seen: true,
        waited_ms: 100,
        since_request_ms: 100,
        manager_state: state,
        result,
    }
}

#[test]
fn an_idle_manager_is_asked_to_reload_once() {
    assert_eq!(
        reload_progress(inputs(false, 0, 0)),
        ReloadProgress::Request
    );
}

#[test]
fn a_busy_manager_is_waited_for_before_and_after_the_request() {
    assert_eq!(reload_progress(inputs(false, 2, 0)), ReloadProgress::Wait);
    assert_eq!(reload_progress(inputs(true, 1, 0)), ReloadProgress::Wait);
    assert_eq!(reload_progress(inputs(true, 2, 0)), ReloadProgress::Wait);
}

#[test]
fn a_finished_reload_is_loaded_when_the_result_is_zero_and_failed_otherwise() {
    assert_eq!(reload_progress(inputs(true, 0, 0)), ReloadProgress::Loaded);
    assert_eq!(
        reload_progress(inputs(true, 0, 0x11)),
        ReloadProgress::Failed(0x11)
    );
}

#[test]
fn an_idle_manager_right_after_the_request_is_not_a_finished_reload() {
    let before_the_read_starts = ReloadInputs {
        busy_seen: false,
        since_request_ms: 10,
        ..inputs(true, 0, 0)
    };
    assert_eq!(
        reload_progress(before_the_read_starts),
        ReloadProgress::Wait
    );
    let read_never_started = ReloadInputs {
        since_request_ms: RELOAD_BUSY_GRACE_MS,
        ..before_the_read_starts
    };
    assert_eq!(reload_progress(read_never_started), ReloadProgress::Loaded);
}

#[test]
fn a_reload_that_never_finishes_times_out() {
    let stuck = ReloadInputs {
        waited_ms: RELOAD_TIMEOUT_MS,
        ..inputs(true, 1, 0)
    };
    assert_eq!(reload_progress(stuck), ReloadProgress::TimedOut);
}

use automata_game::address::{LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION, SCENE_STATE_VALUE};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS, BootLoadEndAction, BootLoadPass,
    BootScreenInputs, BootScreenKeep, BootScreenState, LOAD_DONE_NOT_REQUESTED,
    LOAD_DONE_REQUESTED, LOADING_SCREEN_ANSWERED, LOADING_SCREEN_NOT_ANSWERED, LoaderReading,
    SCENE_LOADER_LAST_STAGE, SCENE_STATE_WHILE_A_LOAD_RUNS, STILL_LOADING_TEST_CODE_CHECKED_LEN,
    STILL_LOADING_TEST_SCENE_STATE_COMPARE_OFFSET, boot_load_end_action,
    still_loading_test_code_is_the_noted_one,
};
use automata_game::start::RUNNING_SCENE_STATE;

const MAIN_STILL_LOADING_TEST: [u8; 0x77] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0xD2, 0x0B, 0x11, 0x00, 0x85, 0xC0,
    0x75, 0x5D, 0x39, 0x83, 0x90, 0x01, 0x00, 0x00, 0x75, 0x4A, 0x83, 0x3D, 0x67, 0x4C, 0x77, 0x00,
    0x01, 0x7C, 0x41, 0xF6, 0x05, 0xF6, 0x43, 0x77, 0x00, 0x80, 0x75, 0x38, 0x39, 0x83, 0x9C, 0x01,
    0x00, 0x00, 0x75, 0x30, 0xE8, 0x07, 0x0B, 0x11, 0x00, 0x85, 0xC0, 0x75, 0x27, 0x83, 0xBB, 0x84,
    0x01, 0x00, 0x00, 0x02, 0x7D, 0x1E, 0xE8, 0xD5, 0x60, 0x88, 0xFF, 0x85, 0xC0, 0x75, 0x15, 0xE8,
    0xAC, 0xFE, 0xF0, 0xFF, 0x85, 0xC0, 0x75, 0x0C, 0xF7, 0x05, 0xF2, 0x43, 0x77, 0x00, 0x00, 0x00,
    0x00, 0x80, 0x74, 0x0B, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3, 0x33,
    0xC0, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3,
];
const COMPAT_STILL_LOADING_TEST: [u8; 0x77] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8, 0x22, 0x0B, 0x11, 0x00, 0x85, 0xC0,
    0x75, 0x5D, 0x39, 0x83, 0x90, 0x01, 0x00, 0x00, 0x75, 0x4A, 0x83, 0x3D, 0xE7, 0x12, 0x77, 0x00,
    0x01, 0x7C, 0x41, 0xF6, 0x05, 0x76, 0x0A, 0x77, 0x00, 0x80, 0x75, 0x38, 0x39, 0x83, 0x9C, 0x01,
    0x00, 0x00, 0x75, 0x30, 0xE8, 0x57, 0x0A, 0x11, 0x00, 0x85, 0xC0, 0x75, 0x27, 0x83, 0xBB, 0x84,
    0x01, 0x00, 0x00, 0x02, 0x7D, 0x1E, 0xE8, 0xD5, 0xE7, 0x88, 0xFF, 0x85, 0xC0, 0x75, 0x15, 0xE8,
    0x6C, 0x03, 0xF1, 0xFF, 0x85, 0xC0, 0x75, 0x0C, 0xF7, 0x05, 0x72, 0x0A, 0x77, 0x00, 0x00, 0x00,
    0x00, 0x80, 0x74, 0x0B, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3, 0x33,
    0xC0, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3,
];

fn kept() -> BootScreenKeep {
    BootScreenKeep::new(true, true, true).0
}

fn at_the_title(now_ms: u64) -> BootScreenInputs {
    BootScreenInputs {
        now_ms,
        boot_load_passed: true,
        next_load_asked: false,
        start_is_over: false,
        scene_state: RUNNING_SCENE_STATE,
    }
}

fn asked_for_the_end() -> LoaderReading {
    LoaderReading {
        stage: Some(SCENE_LOADER_LAST_STAGE),
        load_done_request: Some(LOAD_DONE_REQUESTED),
        loading_screen_answer: Some(LOADING_SCREEN_NOT_ANSWERED),
        loader_finished: false,
    }
}

fn noted(code: &[u8], variant: ExeVariant) -> bool {
    still_loading_test_code_is_the_noted_one(
        code,
        LOADING_SCREEN_STILL_LOADING_TEST_FUNCTION.rva(variant),
        SCENE_STATE_VALUE.rva(variant),
    )
}

#[test]
fn the_screen_is_kept_only_for_a_start_that_skips_the_title_with_the_setting_on_and_the_hooks_in() {
    let (keep, logs) = BootScreenKeep::new(true, true, true);
    assert!(keep.wants_the_screen_kept());
    assert!(logs[0].contains("one loading screen"));
    let (by_hand, logs) = BootScreenKeep::new(true, false, true);
    assert_eq!(by_hand.state(), BootScreenState::NotKept);
    assert!(logs.is_empty());
    let (setting_off, logs) = BootScreenKeep::new(false, true, true);
    assert!(!setting_off.wants_the_screen_kept());
    assert!(logs[0].contains("one_loading_screen is off"));
    let (no_hooks, logs) = BootScreenKeep::new(true, true, false);
    assert!(!no_hooks.wants_the_screen_kept());
    assert!(logs[0].contains("could not be hooked"));
    assert!(!BootScreenKeep::unused().wants_the_screen_kept());
}

#[test]
fn the_screen_stays_kept_over_the_title_and_while_the_old_scene_is_taken_down() {
    let mut keep = kept();
    let before_the_boot_load_ends = BootScreenInputs {
        boot_load_passed: false,
        scene_state: SCENE_STATE_WHILE_A_LOAD_RUNS,
        ..at_the_title(1_000)
    };
    assert!(keep.step(&before_the_boot_load_ends).is_empty());
    assert!(keep.step(&at_the_title(20_000)).is_empty());
    for scene_state in [2, 3, 4] {
        let taken_down = BootScreenInputs {
            next_load_asked: true,
            scene_state,
            ..at_the_title(21_000)
        };
        assert!(keep.step(&taken_down).is_empty());
        assert!(keep.wants_the_screen_kept());
    }
}

#[test]
fn the_keeping_ends_once_the_load_of_the_start_runs() {
    let mut keep = kept();
    keep.step(&at_the_title(20_000));
    let loading = BootScreenInputs {
        next_load_asked: true,
        scene_state: SCENE_STATE_WHILE_A_LOAD_RUNS,
        ..at_the_title(23_000)
    };
    let logs = keep.step(&loading);
    assert!(logs[0].contains("runs under the boot's loading screen"));
    assert_eq!(keep.state(), BootScreenState::Over);
    assert!(!keep.wants_the_screen_kept());
    assert!(keep.step(&loading).is_empty());
}

#[test]
fn a_start_that_ends_at_the_title_lets_the_screen_go() {
    let mut keep = kept();
    let refused = BootScreenInputs {
        start_is_over: true,
        ..at_the_title(20_000)
    };
    let logs = keep.step(&refused);
    assert!(logs[0].contains("the loading screen is let go"));
    assert!(!keep.wants_the_screen_kept());
}

#[test]
fn the_screen_is_let_go_when_no_load_follows_the_boot_load_in_time() {
    let mut keep = kept();
    keep.step(&at_the_title(20_000));
    let just_before = 20_000 + BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS - 1;
    assert!(keep.step(&at_the_title(just_before)).is_empty());
    let logs = keep.step(&at_the_title(just_before + 1));
    assert!(logs[0].contains("no load followed"));
    assert_eq!(keep.state(), BootScreenState::Over);
}

#[test]
fn the_limit_does_not_count_before_the_boot_load_has_ended() {
    let mut keep = kept();
    let still_booting = BootScreenInputs {
        boot_load_passed: false,
        scene_state: SCENE_STATE_WHILE_A_LOAD_RUNS,
        ..at_the_title(10 * BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS)
    };
    assert!(keep.step(&still_booting).is_empty());
    assert!(keep.wants_the_screen_kept());
}

#[test]
fn the_first_load_end_asked_while_the_screen_is_wanted_kept_is_passed() {
    let action =
        |pass, wanted, reading: LoaderReading| boot_load_end_action(pass, wanted, &reading);
    assert_eq!(
        action(BootLoadPass::NotYet, true, asked_for_the_end()),
        BootLoadEndAction::PassTheLoadKeepTheScreen
    );
    assert_eq!(
        action(BootLoadPass::NotYet, false, asked_for_the_end()),
        BootLoadEndAction::Nothing
    );
    let earlier_stage = LoaderReading {
        stage: Some(3),
        load_done_request: Some(LOAD_DONE_NOT_REQUESTED),
        ..asked_for_the_end()
    };
    let answered = LoaderReading {
        loading_screen_answer: Some(LOADING_SCREEN_ANSWERED),
        ..asked_for_the_end()
    };
    let finished = LoaderReading {
        loader_finished: true,
        ..asked_for_the_end()
    };
    for reading in [earlier_stage, answered, finished, LoaderReading::default()] {
        assert_eq!(
            action(BootLoadPass::NotYet, true, reading),
            BootLoadEndAction::Nothing
        );
    }
}

#[test]
fn the_lent_answer_is_taken_back_when_the_loader_has_ended_and_never_given_twice() {
    let lent_and_waiting = LoaderReading {
        load_done_request: Some(LOAD_DONE_NOT_REQUESTED),
        loading_screen_answer: Some(LOADING_SCREEN_ANSWERED),
        ..asked_for_the_end()
    };
    assert_eq!(
        boot_load_end_action(BootLoadPass::AnswerLent, true, &lent_and_waiting),
        BootLoadEndAction::Nothing
    );
    let ended = LoaderReading {
        loader_finished: true,
        ..lent_and_waiting
    };
    assert_eq!(
        boot_load_end_action(BootLoadPass::AnswerLent, false, &ended),
        BootLoadEndAction::TakeTheAnswerBack
    );
    let stage_moved_on = LoaderReading {
        stage: Some(0),
        ..lent_and_waiting
    };
    assert_eq!(
        boot_load_end_action(BootLoadPass::AnswerLent, true, &stage_moved_on),
        BootLoadEndAction::TakeTheAnswerBack
    );
    for reading in [asked_for_the_end(), ended] {
        assert_eq!(
            boot_load_end_action(BootLoadPass::Passed, true, &reading),
            BootLoadEndAction::Nothing
        );
    }
}

#[test]
fn the_still_loading_test_of_both_exes_is_the_noted_code() {
    assert_eq!(
        MAIN_STILL_LOADING_TEST.len(),
        STILL_LOADING_TEST_CODE_CHECKED_LEN
    );
    assert!(noted(&MAIN_STILL_LOADING_TEST, ExeVariant::Main));
    assert!(noted(&COMPAT_STILL_LOADING_TEST, ExeVariant::Compat));
    assert!(!noted(&MAIN_STILL_LOADING_TEST, ExeVariant::Compat));
    assert!(!noted(&COMPAT_STILL_LOADING_TEST, ExeVariant::Main));
}

#[test]
fn still_loading_test_code_that_differs_where_checked_is_refused() {
    let mut other_start = MAIN_STILL_LOADING_TEST;
    other_start[0] = 0x48;
    let mut other_end = MAIN_STILL_LOADING_TEST;
    other_end[STILL_LOADING_TEST_CODE_CHECKED_LEN - 1] = 0xCC;
    let mut other_scene_state = MAIN_STILL_LOADING_TEST;
    other_scene_state[STILL_LOADING_TEST_SCENE_STATE_COMPARE_OFFSET + 2] ^= 4;
    for code in [other_start, other_end, other_scene_state] {
        assert!(!noted(&code, ExeVariant::Main));
    }
    assert!(!noted(&MAIN_STILL_LOADING_TEST[..0x70], ExeVariant::Main));
}

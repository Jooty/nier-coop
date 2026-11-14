use automata_game::address::{
    SYSTEM_DATA_STAGE_VALUE, SYSTEM_DIALOG_OPEN_FUNCTION, TITLE_SYSTEM_DATA_STEP_FUNCTION,
};
use automata_game::exe::ExeVariant;
use automata_game::load_gate::{
    BootScreenDialogs, SYSTEM_DATA_STAGE_WHILE_LOADING,
    SYSTEM_DIALOG_LINES_WHILE_THE_BOOT_SCREEN_IS_KEPT_AT_MOST, SYSTEM_DIALOG_OPEN_FUNCTION_START,
    SystemDialogAnswer, TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN,
    TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET, TITLE_SYSTEM_DATA_STEP_FUNCTION_START,
    system_dialog_open_code_is_the_noted_one, title_system_data_step_code_is_the_noted_one,
};

const MAIN_DIALOG_OPEN_CALL: [u8; 5] = [0xE8, 0x80, 0x66, 0x95, 0xFF];
const COMPAT_DIALOG_OPEN_CALL: [u8; 5] = [0xE8, 0x00, 0x95, 0x95, 0xFF];
const AFTER_THE_DIALOG_OPEN_CALL: [u8; 9] = [0x89, 0x87, 0x90, 0x09, 0x00, 0x00, 0x83, 0xF8, 0xFF];
const SYSTEM_DATA_STAGE_WHILE_SAVING: i32 = 5;

fn step_code(call: [u8; 5]) -> Vec<u8> {
    let mut code = vec![0xCC; TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN];
    code[..TITLE_SYSTEM_DATA_STEP_FUNCTION_START.len()]
        .copy_from_slice(&TITLE_SYSTEM_DATA_STEP_FUNCTION_START);
    let at = TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET;
    code[at..at + 5].copy_from_slice(&call);
    code[at + 5..at + 14].copy_from_slice(&AFTER_THE_DIALOG_OPEN_CALL);
    code
}

fn noted(code: &[u8], variant: ExeVariant) -> bool {
    title_system_data_step_code_is_the_noted_one(
        code,
        TITLE_SYSTEM_DATA_STEP_FUNCTION.rva(variant),
        SYSTEM_DIALOG_OPEN_FUNCTION.rva(variant),
    )
}

#[test]
fn the_band_addresses_are_the_ones_read_in_both_exes() {
    assert_eq!(
        TITLE_SYSTEM_DATA_STEP_FUNCTION.rva(ExeVariant::Main),
        0x99_E0B0
    );
    assert_eq!(
        TITLE_SYSTEM_DATA_STEP_FUNCTION.rva(ExeVariant::Compat),
        0x99_5800
    );
    assert_eq!(SYSTEM_DIALOG_OPEN_FUNCTION.rva(ExeVariant::Main), 0x2F_49A0);
    assert_eq!(
        SYSTEM_DIALOG_OPEN_FUNCTION.rva(ExeVariant::Compat),
        0x2E_EF70
    );
    assert_eq!(SYSTEM_DATA_STAGE_VALUE.rva(ExeVariant::Main), 0x142_2298);
    assert_eq!(SYSTEM_DATA_STAGE_VALUE.rva(ExeVariant::Compat), 0x141_6118);
}

#[test]
fn the_step_of_each_exe_calls_its_own_dialog_opener() {
    assert!(noted(&step_code(MAIN_DIALOG_OPEN_CALL), ExeVariant::Main));
    assert!(noted(
        &step_code(COMPAT_DIALOG_OPEN_CALL),
        ExeVariant::Compat
    ));
    assert!(!noted(
        &step_code(MAIN_DIALOG_OPEN_CALL),
        ExeVariant::Compat
    ));
    assert!(!noted(
        &step_code(COMPAT_DIALOG_OPEN_CALL),
        ExeVariant::Main
    ));
}

#[test]
fn a_step_that_differs_is_not_hooked() {
    let mut other_start = step_code(MAIN_DIALOG_OPEN_CALL);
    other_start[0x18] = 0x90;
    assert!(!noted(&other_start, ExeVariant::Main));

    let mut other_store = step_code(MAIN_DIALOG_OPEN_CALL);
    other_store[TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN - 1] = 0x00;
    assert!(!noted(&other_store, ExeVariant::Main));

    let mut no_call = step_code(MAIN_DIALOG_OPEN_CALL);
    no_call[TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET] = 0x90;
    assert!(!noted(&no_call, ExeVariant::Main));

    let short =
        step_code(MAIN_DIALOG_OPEN_CALL)[..TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN - 1].to_vec();
    assert!(!noted(&short, ExeVariant::Main));
}

#[test]
fn the_dialog_opener_start_is_checked() {
    assert!(system_dialog_open_code_is_the_noted_one(
        &SYSTEM_DIALOG_OPEN_FUNCTION_START
    ));
    let mut other = SYSTEM_DIALOG_OPEN_FUNCTION_START.to_vec();
    other[0x16] = 0x04;
    assert!(!system_dialog_open_code_is_the_noted_one(&other));
    assert!(!system_dialog_open_code_is_the_noted_one(
        &SYSTEM_DIALOG_OPEN_FUNCTION_START[..0x20]
    ));
}

#[test]
fn without_the_kept_screen_every_dialog_opens_and_nothing_is_logged() {
    let mut dialogs = BootScreenDialogs::default();
    let from_the_step = dialogs.ask(false, true, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    assert_eq!(from_the_step, (SystemDialogAnswer::LetItOpen, None));
    let elsewhere = dialogs.ask(false, false, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    assert_eq!(elsewhere, (SystemDialogAnswer::LetItOpen, None));
    assert_eq!(dialogs.held_back(), 0);
}

#[test]
fn the_title_band_is_held_back_under_the_kept_screen_and_logged_once() {
    let mut dialogs = BootScreenDialogs::default();
    let (answer, line) = dialogs.ask(true, true, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    assert_eq!(answer, SystemDialogAnswer::HoldItBack);
    let line = line.unwrap_or_default();
    assert!(line.contains("CORE_SYS_DIALOG_01"), "{line}");
    assert!(line.contains("held back"), "{line}");
    // The title asks again on every frame of the load.
    let again = dialogs.ask(true, true, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    assert_eq!(again, (SystemDialogAnswer::HoldItBack, None));
    assert_eq!(dialogs.held_back(), 2);
}

#[test]
fn the_saving_band_of_the_same_step_is_held_back_with_its_own_line() {
    let mut dialogs = BootScreenDialogs::default();
    let _ = dialogs.ask(true, true, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    let (answer, line) = dialogs.ask(true, true, Some(SYSTEM_DATA_STAGE_WHILE_SAVING));
    assert_eq!(answer, SystemDialogAnswer::HoldItBack);
    assert!(line.unwrap_or_default().contains("CORE_SYS_DIALOG_02"));
    let (answer, line) = dialogs.ask(true, true, None);
    assert_eq!(answer, SystemDialogAnswer::HoldItBack);
    assert_eq!(line, None);
}

#[test]
fn other_dialogs_under_the_kept_screen_open_and_are_named_a_few_times() {
    let mut dialogs = BootScreenDialogs::default();
    for _ in 0..SYSTEM_DIALOG_LINES_WHILE_THE_BOOT_SCREEN_IS_KEPT_AT_MOST {
        let (answer, line) = dialogs.ask(true, false, Some(0));
        assert_eq!(answer, SystemDialogAnswer::LetItOpen);
        assert!(
            line.unwrap_or_default()
                .contains("outside the title's system data step")
        );
    }
    assert_eq!(
        dialogs.ask(true, false, Some(0)),
        (SystemDialogAnswer::LetItOpen, None)
    );
    assert_eq!(dialogs.held_back(), 0);
}

#[test]
fn the_summary_counts_what_was_held_back_and_shown() {
    let mut dialogs = BootScreenDialogs::default();
    for _ in 0..3 {
        let _ = dialogs.ask(true, true, Some(SYSTEM_DATA_STAGE_WHILE_LOADING));
    }
    let _ = dialogs.ask(true, false, None);
    let summary = dialogs.summary_when_the_keep_ends();
    assert!(summary.contains("held back 3 times"), "{summary}");
    assert!(summary.contains("1 other system dialogs"), "{summary}");
}

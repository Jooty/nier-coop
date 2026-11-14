use automata_game::address::{
    BODY_UPDATE_FUNCTION, BODY_UPDATE_INPUT_MODE_COPY, KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BODY_CONTROLLER_OFFSET, BODY_UPDATE_FUNCTION_START, CONTROLLER_KEYBOARD_ENABLE_OFFSET,
    CONTROLLER_MOUSE_ENABLE_OFFSET, InputModeKept, PAD_INPUT_MODE,
    body_update_copies_the_input_mode, code_starts_with,
};

const KEYBOARD_AND_MOUSE_MODE: u32 = 1;
const HERO_BODY: usize = 0x1000;
const HELD_BODY: usize = 0x2000;
const MAIN_EXE_COPY: [u8; 18] = [
    0x8B, 0x05, 0x39, 0x22, 0xFD, 0x00, 0x89, 0x87, 0xC8, 0x13, 0x00, 0x00, 0x89, 0x87, 0xC4, 0x13,
    0x00, 0x00,
];
const COMPAT_EXE_COPY: [u8; 18] = [
    0x8B, 0x05, 0x8D, 0x37, 0xFB, 0x00, 0x89, 0x87, 0xC8, 0x13, 0x00, 0x00, 0x89, 0x87, 0xC4, 0x13,
    0x00, 0x00,
];

fn copy_matches(variant: ExeVariant, bytes: &[u8; 18]) -> bool {
    body_update_copies_the_input_mode(
        bytes,
        BODY_UPDATE_INPUT_MODE_COPY.rva(variant),
        KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE.rva(variant),
    )
}

#[test]
fn the_held_body_updates_in_pad_mode_and_the_mode_goes_back_after() {
    let kept = InputModeKept::for_body_update(HELD_BODY, Some(HELD_BODY), KEYBOARD_AND_MOUSE_MODE)
        .expect("the held body's update is the one that is changed");
    assert_eq!(kept.mode_during_the_update(), PAD_INPUT_MODE);
    assert_eq!(
        kept.mode_to_put_back(PAD_INPUT_MODE),
        Some(KEYBOARD_AND_MOUSE_MODE)
    );
}

#[test]
fn the_local_hero_keeps_its_keyboard_and_mouse() {
    assert_eq!(
        InputModeKept::for_body_update(HERO_BODY, Some(HELD_BODY), KEYBOARD_AND_MOUSE_MODE),
        None
    );
}

#[test]
fn nothing_is_changed_while_no_body_is_held() {
    assert_eq!(
        InputModeKept::for_body_update(HELD_BODY, None, KEYBOARD_AND_MOUSE_MODE),
        None
    );
}

#[test]
fn a_game_already_in_pad_mode_is_left_alone() {
    assert_eq!(
        InputModeKept::for_body_update(HELD_BODY, Some(HELD_BODY), PAD_INPUT_MODE),
        None
    );
}

#[test]
fn a_mode_the_game_wrote_during_the_update_is_not_overwritten() {
    let kept = InputModeKept::for_body_update(HELD_BODY, Some(HELD_BODY), KEYBOARD_AND_MOUSE_MODE)
        .expect("the held body's update is the one that is changed");
    assert_eq!(kept.mode_to_put_back(KEYBOARD_AND_MOUSE_MODE), None);
}

#[test]
fn the_copy_in_both_exes_is_the_noted_one() {
    assert!(copy_matches(ExeVariant::Main, &MAIN_EXE_COPY));
    assert!(copy_matches(ExeVariant::Compat, &COMPAT_EXE_COPY));
}

#[test]
fn a_copy_from_another_global_or_into_other_fields_does_not_match() {
    assert!(!copy_matches(ExeVariant::Main, &COMPAT_EXE_COPY));
    assert!(!copy_matches(ExeVariant::Compat, &MAIN_EXE_COPY));
    let mut other_field = MAIN_EXE_COPY;
    other_field[8] = 0xC0;
    assert!(!copy_matches(ExeVariant::Main, &other_field));
    let mut other_instruction = MAIN_EXE_COPY;
    other_instruction[0] = 0x8D;
    assert!(!copy_matches(ExeVariant::Main, &other_instruction));
}

#[test]
fn the_copy_writes_the_two_enables_the_hold_names() {
    let keyboard_enable = BODY_CONTROLLER_OFFSET + CONTROLLER_KEYBOARD_ENABLE_OFFSET;
    let mouse_enable = BODY_CONTROLLER_OFFSET + CONTROLLER_MOUSE_ENABLE_OFFSET;
    assert_eq!(MAIN_EXE_COPY[8..12], keyboard_enable.to_le_bytes());
    assert_eq!(MAIN_EXE_COPY[14..18], mouse_enable.to_le_bytes());
}

#[test]
fn the_body_update_and_its_copy_are_picked_per_exe() {
    let expected = [
        (BODY_UPDATE_FUNCTION, 0x42A190, 0x43CAD0),
        (BODY_UPDATE_INPUT_MODE_COPY, 0x42A1C5, 0x43CB05),
        (KEYBOARD_AND_MOUSE_INPUT_MODE_VALUE, 0x13FC404, 0x13F0298),
    ];
    for (address, main, compat) in expected {
        assert_eq!(address.rva(ExeVariant::Main), main);
        assert_eq!(address.rva(ExeVariant::Compat), compat);
    }
}

#[test]
fn the_body_update_start_matches_only_its_noted_bytes() {
    let mut found = BODY_UPDATE_FUNCTION_START.to_vec();
    assert!(code_starts_with(&found, &BODY_UPDATE_FUNCTION_START));
    found[2] ^= 1;
    assert!(!code_starts_with(&found, &BODY_UPDATE_FUNCTION_START));
}

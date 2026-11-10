use automata_game::address::GAME_FOCUS_LOST_TEST_FUNCTION;
use automata_game::exe::ExeVariant;
use automata_game::focus::{FocusAnswer, GAME_FOCUS_LOST_TEST_FUNCTION_START, focus_test_answer};

#[test]
fn a_linked_game_is_told_it_has_focus_and_the_pass_is_noted() {
    assert_eq!(
        focus_test_answer(true, true),
        FocusAnswer {
            focus_lost: false,
            passed_over: true
        }
    );
}

#[test]
fn a_game_that_is_not_linked_keeps_its_own_answer() {
    assert_eq!(
        focus_test_answer(true, false),
        FocusAnswer {
            focus_lost: true,
            passed_over: false
        }
    );
}

#[test]
fn a_game_with_focus_is_never_passed_over() {
    for linked in [false, true] {
        assert_eq!(
            focus_test_answer(false, linked),
            FocusAnswer {
                focus_lost: false,
                passed_over: false
            }
        );
    }
}

#[test]
fn the_focus_test_is_picked_per_exe_and_starts_with_two_whole_instructions() {
    assert_eq!(
        GAME_FOCUS_LOST_TEST_FUNCTION.rva(ExeVariant::Main),
        0x9C_6250
    );
    assert_eq!(
        GAME_FOCUS_LOST_TEST_FUNCTION.rva(ExeVariant::Compat),
        0x9B_D9A0
    );
    assert_eq!(
        GAME_FOCUS_LOST_TEST_FUNCTION_START,
        [0x48, 0x83, 0xEC, 0x28, 0xB9, 0x20, 0x00, 0x00, 0x00]
    );
}

mod focus_rule;

pub use focus_rule::{FocusAnswer, focus_test_answer};

pub const GAME_FOCUS_LOST_TEST_FUNCTION_START: [u8; 9] =
    [0x48, 0x83, 0xEC, 0x28, 0xB9, 0x20, 0x00, 0x00, 0x00];

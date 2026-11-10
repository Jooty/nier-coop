/// What the game's own "the window lost focus" test is made to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusAnswer {
    pub focus_lost: bool,
    pub passed_over: bool,
}

/// While a co-op session is linked the game is told it has focus, so its pause does not open.
pub fn focus_test_answer(game_says_focus_lost: bool, session_linked: bool) -> FocusAnswer {
    let passed_over = game_says_focus_lost && session_linked;
    FocusAnswer {
        focus_lost: game_says_focus_lost && !passed_over,
        passed_over,
    }
}

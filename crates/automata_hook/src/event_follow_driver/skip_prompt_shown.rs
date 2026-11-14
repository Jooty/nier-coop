use std::sync::Mutex;

use automata_game::event_follow::SkipPrompt;

static SHOWN_SKIP_PROMPT: Mutex<Option<SkipPrompt>> = Mutex::new(None);

pub(super) fn set_skip_prompt(prompt: Option<SkipPrompt>) {
    if let Ok(mut shown) = SHOWN_SKIP_PROMPT.lock() {
        *shown = prompt;
    }
}

/// The skip vote line to draw this frame, if any.
pub fn shown_skip_prompt() -> Option<SkipPrompt> {
    SHOWN_SKIP_PROMPT.lock().ok().and_then(|shown| *shown)
}

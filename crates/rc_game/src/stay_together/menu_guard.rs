use crate::stay_together::FOCUS_SETTLE_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuGuard {
    was_unfocused: bool,
    open_at_focus_loss: bool,
    settle_until_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuGuardInput {
    pub keep_running: bool,
    pub unfocused: bool,
    pub menu_open: bool,
    pub now_ms: u64,
}

impl MenuGuard {
    pub const fn new() -> Self {
        Self {
            was_unfocused: false,
            open_at_focus_loss: false,
            settle_until_ms: 0,
        }
    }

    pub fn should_close_menu(&mut self, input: &MenuGuardInput) -> bool {
        if input.unfocused && !self.was_unfocused {
            self.open_at_focus_loss = input.menu_open;
        }
        if !input.unfocused && self.was_unfocused {
            self.settle_until_ms = input.now_ms + FOCUS_SETTLE_MS;
        }
        self.was_unfocused = input.unfocused;
        let around_focus_change = input.unfocused || input.now_ms < self.settle_until_ms;
        input.keep_running && input.menu_open && around_focus_change && !self.open_at_focus_loss
    }
}

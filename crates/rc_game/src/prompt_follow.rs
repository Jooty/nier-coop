pub const PROMPT_PAGE_SHOWN_DIALOG_STATE: i32 = 0x5;
pub const MAX_PROMPT_CLOSE_TRIES_PER_PAGE: u32 = 3;
pub const PROMPT_CLOSE_RETRY_AFTER_MS: u64 = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptView {
    pub stream_active: bool,
    pub dialog_state: i32,
    pub pause_menu_open: bool,
    pub now_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptClose {
    pub page: u32,
    pub host_dismissed: u32,
    pub try_number: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptStep {
    pub send_count: Option<u32>,
    pub close: Option<PromptClose>,
    pub gave_up_on_page: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CloseAttempt {
    tries: u32,
    last_try_ms: u64,
    gave_up_reported: bool,
}

#[derive(Clone, Debug, Default)]
pub struct PromptFollow {
    stream_was_active: bool,
    dismissed: u32,
    page_shown: bool,
    host_dismissed: u32,
    last_sent: u32,
    attempt: Option<CloseAttempt>,
}

impl PromptFollow {
    pub const fn new() -> Self {
        Self {
            stream_was_active: false,
            dismissed: 0,
            page_shown: false,
            host_dismissed: 0,
            last_sent: 0,
            attempt: None,
        }
    }

    pub const fn dismissed(&self) -> u32 {
        self.dismissed
    }

    pub const fn host_dismissed(&self) -> u32 {
        self.host_dismissed
    }

    pub const fn receive_host_count(&mut self, count: u32) {
        self.host_dismissed = count;
    }

    pub fn tick(&mut self, view: PromptView) -> PromptStep {
        let mut step = PromptStep::default();
        if view.stream_active != self.stream_was_active {
            self.stream_was_active = view.stream_active;
            self.dismissed = 0;
            self.page_shown = false;
            self.attempt = None;
            if !view.stream_active {
                self.host_dismissed = 0;
            }
            if self.last_sent != 0 {
                self.last_sent = 0;
                step.send_count = Some(0);
            }
        }
        if !view.stream_active {
            return step;
        }
        let page_open = view.dialog_state == PROMPT_PAGE_SHOWN_DIALOG_STATE;
        let page_shown_now = page_open && !view.pause_menu_open;
        if page_shown_now {
            self.page_shown = true;
        } else if self.page_shown && !page_open {
            self.page_shown = false;
            self.dismissed = self.dismissed.saturating_add(1);
            self.attempt = None;
        }
        if self.dismissed != self.last_sent {
            self.last_sent = self.dismissed;
            step.send_count = Some(self.dismissed);
        }
        if page_shown_now && self.dismissed < self.host_dismissed {
            self.step_close(view.now_ms, &mut step);
        }
        step
    }

    fn step_close(&mut self, now_ms: u64, step: &mut PromptStep) {
        let page = self.dismissed.saturating_add(1);
        let host_dismissed = self.host_dismissed;
        let Some(attempt) = &mut self.attempt else {
            self.attempt = Some(CloseAttempt {
                tries: 1,
                last_try_ms: now_ms,
                gave_up_reported: false,
            });
            step.close = Some(PromptClose {
                page,
                host_dismissed,
                try_number: 1,
            });
            return;
        };
        if now_ms.saturating_sub(attempt.last_try_ms) < PROMPT_CLOSE_RETRY_AFTER_MS {
            return;
        }
        if attempt.tries >= MAX_PROMPT_CLOSE_TRIES_PER_PAGE {
            if !attempt.gave_up_reported {
                attempt.gave_up_reported = true;
                step.gave_up_on_page = Some(page);
            }
            return;
        }
        attempt.tries += 1;
        attempt.last_try_ms = now_ms;
        step.close = Some(PromptClose {
            page,
            host_dismissed,
            try_number: attempt.tries,
        });
    }
}

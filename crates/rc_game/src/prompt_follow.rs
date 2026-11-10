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
pub struct PromptPageId {
    pub kind: i32,
    pub id: i32,
    pub page: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptDismissal {
    pub page: PromptPageId,
    pub times: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptClose {
    pub page: PromptPageId,
    pub try_number: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptStep {
    pub dismissed: Option<PromptDismissal>,
    pub close: Option<PromptClose>,
    pub gave_up_on: Option<PromptPageId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CloseAttempt {
    tries: u32,
    last_try_ms: u64,
    gave_up_reported: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OpenIdentity {
    kind: i32,
    id: i32,
}

#[derive(Clone, Debug, Default)]
pub struct PromptFollow {
    stream_was_active: bool,
    current: Option<OpenIdentity>,
    pending_open: Option<OpenIdentity>,
    pages_shown: u32,
    page_shown: bool,
    own_dismissed: Vec<(PromptPageId, u32)>,
    host_dismissed: Vec<(PromptPageId, u32)>,
    attempt: Option<CloseAttempt>,
}

fn count_of(counts: &[(PromptPageId, u32)], page: PromptPageId) -> u32 {
    counts
        .iter()
        .find(|(known, _)| *known == page)
        .map_or(0, |(_, times)| *times)
}

fn set_count(counts: &mut Vec<(PromptPageId, u32)>, page: PromptPageId, times: u32) {
    if let Some(entry) = counts.iter_mut().find(|(known, _)| *known == page) {
        entry.1 = times;
    } else {
        counts.push((page, times));
    }
}

impl PromptFollow {
    pub const fn new() -> Self {
        Self {
            stream_was_active: false,
            current: None,
            pending_open: None,
            pages_shown: 0,
            page_shown: false,
            own_dismissed: Vec::new(),
            host_dismissed: Vec::new(),
            attempt: None,
        }
    }

    pub fn dismissed_times(&self, page: PromptPageId) -> u32 {
        count_of(&self.own_dismissed, page)
    }

    pub fn host_dismissed_times(&self, page: PromptPageId) -> u32 {
        count_of(&self.host_dismissed, page)
    }

    pub fn receive_host_dismissal(&mut self, dismissal: PromptDismissal) {
        set_count(&mut self.host_dismissed, dismissal.page, dismissal.times);
    }

    pub const fn note_open(&mut self, kind: i32, id: i32) {
        let identity = OpenIdentity { kind, id };
        if self.page_shown {
            self.pending_open = Some(identity);
        } else {
            self.current = Some(identity);
            self.pages_shown = 0;
        }
    }

    pub fn tick(&mut self, view: PromptView) -> PromptStep {
        let mut step = PromptStep::default();
        if view.stream_active != self.stream_was_active {
            self.stream_was_active = view.stream_active;
            self.own_dismissed.clear();
            self.attempt = None;
            if !view.stream_active {
                self.host_dismissed.clear();
            }
        }
        let page_open = view.dialog_state == PROMPT_PAGE_SHOWN_DIALOG_STATE;
        let page_shown_now = page_open && !view.pause_menu_open;
        if page_shown_now && !self.page_shown {
            self.page_shown = true;
            self.pages_shown = self.pages_shown.saturating_add(1);
        } else if self.page_shown && !page_open {
            self.page_shown = false;
            self.attempt = None;
            step.dismissed = self.count_dismissal(view.stream_active);
        }
        if !self.page_shown
            && let Some(identity) = self.pending_open.take()
        {
            self.current = Some(identity);
            self.pages_shown = 0;
        }
        if view.dialog_state == 0 && !self.page_shown {
            self.current = None;
            self.pages_shown = 0;
        }
        if view.stream_active
            && page_shown_now
            && let Some(page) = self.shown_page()
            && count_of(&self.own_dismissed, page) < count_of(&self.host_dismissed, page)
        {
            self.step_close(page, view.now_ms, &mut step);
        }
        step
    }

    fn shown_page(&self) -> Option<PromptPageId> {
        let identity = self.current?;
        Some(PromptPageId {
            kind: identity.kind,
            id: identity.id,
            page: self.pages_shown,
        })
    }

    fn count_dismissal(&mut self, stream_active: bool) -> Option<PromptDismissal> {
        let page = self.shown_page()?;
        if !stream_active {
            return None;
        }
        let times = count_of(&self.own_dismissed, page).saturating_add(1);
        set_count(&mut self.own_dismissed, page, times);
        Some(PromptDismissal { page, times })
    }

    fn step_close(&mut self, page: PromptPageId, now_ms: u64, step: &mut PromptStep) {
        let Some(attempt) = &mut self.attempt else {
            self.attempt = Some(CloseAttempt {
                tries: 1,
                last_try_ms: now_ms,
                gave_up_reported: false,
            });
            step.close = Some(PromptClose {
                page,
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
                step.gave_up_on = Some(page);
            }
            return;
        }
        attempt.tries += 1;
        attempt.last_try_ms = now_ms;
        step.close = Some(PromptClose {
            page,
            try_number: attempt.tries,
        });
    }
}

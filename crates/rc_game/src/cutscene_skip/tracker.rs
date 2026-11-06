use crate::cutscene_skip::{ASK_RESEND_INTERVAL_MS, PARTNER_ASK_LIFETIME_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipVerdict {
    PassThrough,
    Hold,
    LetGameSkip,
    SkipNow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipStep {
    pub verdict: SkipVerdict,
    pub newly_asked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CutsceneSkipTracker {
    in_scene: bool,
    skip_taken: bool,
    local_asked: bool,
    executed: bool,
    partner_asked_at: Option<u64>,
    partner_was_in_cutscene: bool,
    last_sent_at: Option<u64>,
}

impl CutsceneSkipTracker {
    pub const fn new() -> Self {
        Self {
            in_scene: false,
            skip_taken: false,
            local_asked: false,
            executed: false,
            partner_asked_at: None,
            partner_was_in_cutscene: false,
            last_sent_at: None,
        }
    }

    pub fn observe_scene(&mut self, in_scene: bool, skip_taken: bool) -> bool {
        let new_scene = in_scene != self.in_scene || (self.skip_taken && !skip_taken);
        self.skip_taken = skip_taken;
        if new_scene {
            *self = Self {
                in_scene,
                skip_taken,
                ..Self::new()
            };
        }
        new_scene
    }

    pub fn in_scene(&self) -> bool {
        self.in_scene
    }

    pub fn local_asked(&self) -> bool {
        self.local_asked
    }

    pub fn partner_asked(&mut self, now_ms: u64) -> bool {
        if !self.in_scene {
            return false;
        }
        let was_alive = self.partner_ask_alive(now_ms);
        self.partner_asked_at = Some(now_ms);
        !was_alive
    }

    pub fn partner_left(&mut self) {
        self.partner_asked_at = None;
    }

    pub fn observe_partner_in_cutscene(&mut self, in_cutscene: bool) {
        if self.partner_was_in_cutscene && !in_cutscene {
            self.partner_asked_at = None;
        }
        self.partner_was_in_cutscene = in_cutscene;
    }

    pub fn partner_ask_alive(&self, now_ms: u64) -> bool {
        self.partner_asked_at
            .is_some_and(|at| now_ms.saturating_sub(at) < PARTNER_ASK_LIFETIME_MS)
    }

    pub fn resend_due(&mut self, now_ms: u64) -> bool {
        if !self.in_scene || !self.local_asked {
            return false;
        }
        let due = self
            .last_sent_at
            .is_none_or(|at| now_ms.saturating_sub(at) >= ASK_RESEND_INTERVAL_MS);
        if due {
            self.last_sent_at = Some(now_ms);
        }
        due
    }

    pub fn step(&mut self, pressed: bool, partner_participates: bool, now_ms: u64) -> SkipStep {
        let mut step = SkipStep {
            verdict: SkipVerdict::PassThrough,
            newly_asked: false,
        };
        if !self.in_scene || self.executed {
            return step;
        }
        let first_press = pressed && !self.local_asked;
        if first_press {
            self.local_asked = true;
            step.newly_asked = true;
        }
        if !self.local_asked {
            return step;
        }
        let everyone_asked = self.partner_ask_alive(now_ms) || !partner_participates;
        step.verdict = match (everyone_asked, first_press) {
            (false, _) => SkipVerdict::Hold,
            (true, true) => SkipVerdict::LetGameSkip,
            (true, false) => SkipVerdict::SkipNow,
        };
        if everyone_asked {
            self.executed = true;
        }
        step
    }
}

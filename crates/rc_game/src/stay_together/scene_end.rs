use crate::stay_together::FOLLOW_WAITS_FOR_THE_SCENE_END_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneEndFacts {
    pub in_scene: bool,
    pub mask_free: bool,
    pub slot_idle: bool,
    pub partner_end_waiting: bool,
    pub local_phase: Option<u16>,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneEndOwed {
    No,
    Yes,
    GaveUp { waited_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneEndWatch {
    was_in_scene: bool,
    owed_since_ms: Option<u64>,
    phase_when_owed: Option<u16>,
    gave_up_on_waiting_end: bool,
}

impl SceneEndWatch {
    pub const fn new() -> Self {
        Self {
            was_in_scene: false,
            owed_since_ms: None,
            phase_when_owed: None,
            gave_up_on_waiting_end: false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn observe(&mut self, facts: &SceneEndFacts) -> SceneEndOwed {
        let scene_just_ended = self.was_in_scene && !facts.in_scene;
        self.was_in_scene = facts.in_scene;
        if !facts.partner_end_waiting {
            self.gave_up_on_waiting_end = false;
        }
        if facts.in_scene {
            self.start_owing(facts);
            return SceneEndOwed::Yes;
        }
        let waiting_end_starts_it = facts.partner_end_waiting
            && !self.gave_up_on_waiting_end
            && self.owed_since_ms.is_none();
        if scene_just_ended || waiting_end_starts_it {
            self.start_owing(facts);
        }
        let Some(since_ms) = self.owed_since_ms else {
            return SceneEndOwed::No;
        };
        let phase_changed = matches!(
            (self.phase_when_owed, facts.local_phase),
            (Some(before), Some(now)) if before != now
        );
        let settled = facts.mask_free && facts.slot_idle && !facts.partner_end_waiting;
        if settled || phase_changed {
            self.owed_since_ms = None;
            return SceneEndOwed::No;
        }
        let waited_ms = facts.now_ms.saturating_sub(since_ms);
        if waited_ms >= FOLLOW_WAITS_FOR_THE_SCENE_END_MS {
            self.owed_since_ms = None;
            self.gave_up_on_waiting_end = facts.partner_end_waiting;
            return SceneEndOwed::GaveUp { waited_ms };
        }
        SceneEndOwed::Yes
    }

    fn start_owing(&mut self, facts: &SceneEndFacts) {
        self.owed_since_ms = Some(facts.now_ms);
        self.phase_when_owed = facts.local_phase;
    }
}

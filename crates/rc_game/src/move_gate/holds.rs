use crate::move_gate::{
    MOVE_HOLD_GIVE_UP_MS, MOVE_HOLD_REMINDER_MS, MOVE_RELEASE_PASS_MS, MoveVerdict,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldMove {
    pub name: String,
    pub phase: u16,
    pub after_talk_command: bool,
    since_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveRelease {
    PartnerArrived,
    GaveUp { waited_ms: u64 },
    LeftPhase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveReleased {
    pub held: HeldMove,
    pub why: MoveRelease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartAnswer {
    Run,
    Hold { first: bool },
}

#[derive(Debug, Clone, Default)]
pub struct MoveHolds {
    held: Vec<HeldMove>,
    passes: Vec<(String, u64)>,
    last_reminder_ms: u64,
}

impl MoveHolds {
    pub const fn new() -> Self {
        Self {
            held: Vec::new(),
            passes: Vec::new(),
            last_reminder_ms: 0,
        }
    }

    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    pub fn on_start(
        &mut self,
        name: &str,
        phase: u16,
        after_talk_command: bool,
        verdict: &MoveVerdict,
        now_ms: u64,
    ) -> StartAnswer {
        self.passes
            .retain(|(_, since)| now_ms.saturating_sub(*since) <= MOVE_RELEASE_PASS_MS);
        if let Some(index) = self.passes.iter().position(|(known, _)| known == name) {
            self.passes.remove(index);
            return StartAnswer::Run;
        }
        if *verdict == MoveVerdict::Allow {
            self.held.retain(|held| held.name != name);
            return StartAnswer::Run;
        }
        if self.held.iter().any(|held| held.name == name) {
            return StartAnswer::Hold { first: false };
        }
        if self.held.is_empty() {
            self.last_reminder_ms = now_ms;
        }
        self.held.push(HeldMove {
            name: name.to_owned(),
            phase,
            after_talk_command,
            since_ms: now_ms,
        });
        StartAnswer::Hold { first: true }
    }

    pub fn step(
        &mut self,
        verdict: &MoveVerdict,
        local_phase: Option<u16>,
        now_ms: u64,
    ) -> Vec<MoveReleased> {
        let mut released = Vec::new();
        let mut still_held = Vec::new();
        for held in self.held.drain(..) {
            let waited_ms = now_ms.saturating_sub(held.since_ms);
            let why = if local_phase != Some(held.phase) {
                Some(MoveRelease::LeftPhase)
            } else if *verdict == MoveVerdict::Allow {
                Some(MoveRelease::PartnerArrived)
            } else if waited_ms >= MOVE_HOLD_GIVE_UP_MS {
                Some(MoveRelease::GaveUp { waited_ms })
            } else {
                None
            };
            match why {
                Some(why) => released.push(MoveReleased { held, why }),
                None => still_held.push(held),
            }
        }
        self.held = still_held;
        for item in &released {
            if item.why != MoveRelease::LeftPhase {
                self.passes.push((item.held.name.clone(), now_ms));
            }
        }
        released
    }

    pub fn reminder_due(&mut self, now_ms: u64) -> bool {
        if self.held.is_empty()
            || now_ms.saturating_sub(self.last_reminder_ms) < MOVE_HOLD_REMINDER_MS
        {
            return false;
        }
        self.last_reminder_ms = now_ms;
        true
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

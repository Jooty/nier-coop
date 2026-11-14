use crate::rule_rooms::{RETURN_FADE_OUT_MS, RulePlayer, RuleRoom};

/// The one return in progress on this game: the breaker fades out, is put back at the room's start
/// spot once the fade has covered the screen, and the room's breaks are armed again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BreakerReturn {
    waiting: Option<(&'static RuleRoom, RulePlayer, u64)>,
}

impl BreakerReturn {
    pub const fn new() -> Self {
        Self { waiting: None }
    }

    pub fn is_returning(&self) -> bool {
        self.waiting.is_some()
    }

    /// False when a return is already running: a second break during the fade changes nothing.
    pub fn begin(&mut self, room: &'static RuleRoom, player: RulePlayer, now_ms: u64) -> bool {
        if self.waiting.is_some() {
            return false;
        }
        self.waiting = Some((room, player, now_ms.saturating_add(RETURN_FADE_OUT_MS)));
        true
    }

    pub fn take_if_due(&mut self, now_ms: u64) -> Option<(&'static RuleRoom, RulePlayer)> {
        let (room, player, due_ms) = self.waiting?;
        if now_ms < due_ms {
            return None;
        }
        self.waiting = None;
        Some((room, player))
    }
}

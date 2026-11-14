use crate::rule_rooms::{ROOM_CLOCK_LOG_EVERY_MS, ROOM_CLOCK_SECONDS, RulePlayer};

/// The one clock of a room that resets as one: the host's, started when the room's rule is armed.
/// The guest's own timer never decides; it counts the same span from its own arming to show it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RoomClock {
    started_ms: Option<u64>,
    last_logged_ms: u64,
}

impl RoomClock {
    pub const fn new() -> Self {
        Self {
            started_ms: None,
            last_logged_ms: 0,
        }
    }

    pub fn start(&mut self, now_ms: u64) {
        self.started_ms = Some(now_ms);
        self.last_logged_ms = now_ms;
    }

    pub fn stop(&mut self) {
        self.started_ms = None;
    }

    pub fn remaining_seconds(&self, now_ms: u64) -> Option<u64> {
        let started = self.started_ms?;
        let elapsed = now_ms.saturating_sub(started) / 1000;
        Some(ROOM_CLOCK_SECONDS.saturating_sub(elapsed))
    }

    pub fn line_if_due(&mut self, now_ms: u64, player: RulePlayer) -> Option<String> {
        let remaining = self.remaining_seconds(now_ms)?;
        if now_ms.saturating_sub(self.last_logged_ms) < ROOM_CLOCK_LOG_EVERY_MS {
            return None;
        }
        self.last_logged_ms = now_ms;
        Some(match player {
            RulePlayer::Host => {
                format!("rule room: the room's clock (the host's) has {remaining} s left")
            }
            RulePlayer::Guest => format!(
                "rule room: the room's clock shown here is the host's, {remaining} s left; your own timer never ends the room"
            ),
        })
    }
}

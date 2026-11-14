use crate::guest_watches::WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS;

/// Watches whether the driven hero's state has followed the host's: no state is started on
/// the hero, so a lasting difference is only logged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateDifference {
    differs_since_ms: Option<u64>,
    logged: bool,
}

impl StateDifference {
    /// A line once a difference has lasted longer than the bound, and once when it is gone.
    pub fn observe(&mut self, now_ms: u64, hero_state: i32, host_state: u16) -> Option<String> {
        if hero_state == i32::from(host_state) {
            let lasted_ms = self
                .differs_since_ms
                .take()
                .map(|since| now_ms.saturating_sub(since));
            return core::mem::take(&mut self.logged)
                .then(|| lasted_ms.map(|ms| state_agrees_again_line(hero_state, ms)))
                .flatten();
        }
        let since_ms = *self.differs_since_ms.get_or_insert(now_ms);
        let lasted_ms = now_ms.saturating_sub(since_ms);
        if self.logged || lasted_ms <= WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS {
            return None;
        }
        self.logged = true;
        Some(state_differs_line(hero_state, host_state, lasted_ms))
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

pub fn state_differs_line(hero_state: i32, host_state: u16, lasted_ms: u64) -> String {
    format!(
        "watching: this game's hero is in state {hero_state:#X} and the host's in {host_state:#X} for {lasted_ms} ms (more than {WATCHED_HERO_STATE_MAY_DIFFER_FOR_MS} ms); no state is started on the hero, the fed input and the game make its states"
    )
}

pub fn state_agrees_again_line(state: i32, lasted_ms: u64) -> String {
    format!(
        "watching: this game's hero and the host's are both in state {state:#X} again, after {lasted_ms} ms apart"
    )
}

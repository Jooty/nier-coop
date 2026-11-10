use crate::cube_probe::{
    BOX_MOTION_NAME_PART, GRAB_START_MOTION_NAME_PART, HELD_POSITION_LOG_INTERVAL_MS,
    HOLD_GIVE_UP_AFTER_MS, NON_BOX_MOTION_GIVE_UP_AFTER_MS, RELEASE_MOTION_NAME_PART,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabEvent {
    Idle,
    Started,
    Holding,
    Released,
    GaveUp,
}

#[derive(Debug, Default)]
pub struct GrabWatch {
    held_since_ms: Option<u64>,
    last_logged_ms: u64,
    non_box_since_ms: Option<u64>,
}

impl GrabWatch {
    pub fn holding(&self) -> bool {
        self.held_since_ms.is_some()
    }

    pub fn held_for_ms(&self, now_ms: u64) -> u64 {
        self.held_since_ms
            .map_or(0, |since| now_ms.saturating_sub(since))
    }

    pub fn observe(&mut self, now_ms: u64, motion_name: Option<&str>) -> GrabEvent {
        let name = motion_name.unwrap_or("");
        let Some(since) = self.held_since_ms else {
            if name.contains(GRAB_START_MOTION_NAME_PART) {
                self.held_since_ms = Some(now_ms);
                self.last_logged_ms = now_ms;
                self.non_box_since_ms = None;
                return GrabEvent::Started;
            }
            return GrabEvent::Idle;
        };
        if name.contains(RELEASE_MOTION_NAME_PART) {
            self.held_since_ms = None;
            return GrabEvent::Released;
        }
        if self.gives_up(now_ms, since, name) {
            self.held_since_ms = None;
            return GrabEvent::GaveUp;
        }
        if now_ms.saturating_sub(self.last_logged_ms) >= HELD_POSITION_LOG_INTERVAL_MS {
            self.last_logged_ms = now_ms;
            return GrabEvent::Holding;
        }
        GrabEvent::Idle
    }

    fn gives_up(&mut self, now_ms: u64, since: u64, name: &str) -> bool {
        if now_ms.saturating_sub(since) >= HOLD_GIVE_UP_AFTER_MS {
            return true;
        }
        if name.contains(BOX_MOTION_NAME_PART) {
            self.non_box_since_ms = None;
            return false;
        }
        let first_off = *self.non_box_since_ms.get_or_insert(now_ms);
        now_ms.saturating_sub(first_off) >= NON_BOX_MOTION_GIVE_UP_AFTER_MS
    }
}

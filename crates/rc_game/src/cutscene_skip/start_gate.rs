use crate::cutscene_skip::{SCENE_START_HOLD_RESTART_GAP_MS, SCENE_START_HOLD_TIMEOUT_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartVerdict {
    Start { waited_ms: u64 },
    Hold { first: bool },
    StartAfterTimeout { waited_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SceneStartGate {
    host_entries: u32,
    paired_entries: u32,
    host_in_scene: bool,
    primed: bool,
    holding_since_ms: Option<u64>,
    last_asked_ms: u64,
}

impl SceneStartGate {
    pub const fn new() -> Self {
        Self {
            host_entries: 0,
            paired_entries: 0,
            host_in_scene: false,
            primed: false,
            holding_since_ms: None,
            last_asked_ms: 0,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn observe_host(&mut self, host_in_scene: bool) {
        if self.primed && host_in_scene && !self.host_in_scene {
            self.host_entries += 1;
        }
        self.primed = true;
        self.host_in_scene = host_in_scene;
    }

    pub fn ask(&mut self, now_ms: u64) -> StartVerdict {
        let stale = now_ms.saturating_sub(self.last_asked_ms) > SCENE_START_HOLD_RESTART_GAP_MS;
        self.last_asked_ms = now_ms;
        if stale {
            self.holding_since_ms = None;
        }
        let waited_ms = self
            .holding_since_ms
            .map_or(0, |since| now_ms.saturating_sub(since));
        if self.host_in_scene || self.host_entries > self.paired_entries {
            self.paired_entries = self.host_entries;
            self.holding_since_ms = None;
            return StartVerdict::Start { waited_ms };
        }
        if waited_ms >= SCENE_START_HOLD_TIMEOUT_MS {
            self.holding_since_ms = None;
            return StartVerdict::StartAfterTimeout { waited_ms };
        }
        let first = self.holding_since_ms.is_none();
        if first {
            self.holding_since_ms = Some(now_ms);
        }
        StartVerdict::Hold { first }
    }
}

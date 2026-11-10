pub const BACKUP_SAVE_DATA_RVA: u32 = 0x3B_BC10;
pub const RESTORE_SAVE_DATA_RVA: u32 = 0x3B_DB70;
pub const RESTART_PHASE_RVA: u32 = 0x41_EAE0;
pub const MAP_MANAGER_RVA: u32 = 0x484_CF20;
pub const GAME_OVER_MESSAGE: u32 = 210;
pub const GUEST_WAITS_FOR_HOST_GAME_OVER_SECONDS: f32 = 10.0;
pub const BACKUP_WAITS_FOR_PLAY_SECONDS: f32 = 10.0;
pub const REWIND_WAITS_FOR_PLAY_SECONDS: f32 = 10.0;
pub const HEAL_WATCH_GIVES_UP_SECONDS: f32 = 30.0;
pub const GAME_OVER_FLOW_WINDOW_MS: u64 = 30_000;
pub const OWN_DRIFT_HOLD_AFTER_REWIND_MS: u64 = 3_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewindDrop {
    NoCheckpoint,
    AlreadyRewinding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostGameOverVerdict {
    Rewind,
    Drop(RewindDrop),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RewindRules {
    checkpoint_taken: bool,
    rewinding: bool,
    host_wait_seconds: Option<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostGameOverGate {
    armed_at_ms: Option<u64>,
}

impl RewindRules {
    pub const fn new() -> Self {
        Self {
            checkpoint_taken: false,
            rewinding: false,
            host_wait_seconds: None,
        }
    }

    pub fn checkpoint_taken(&self) -> bool {
        self.checkpoint_taken
    }

    pub fn is_rewinding(&self) -> bool {
        self.rewinding
    }

    pub fn is_waiting_for_host(&self) -> bool {
        self.host_wait_seconds.is_some()
    }

    pub fn mark_checkpoint_taken(&mut self) {
        self.checkpoint_taken = true;
    }

    pub fn start_waiting_for_host(&mut self) {
        self.host_wait_seconds.get_or_insert(0.0);
    }

    pub fn stop_waiting_for_host(&mut self) {
        self.host_wait_seconds = None;
    }

    pub fn wait_is_over(&mut self, dt_seconds: f32) -> bool {
        let Some(waited) = self.host_wait_seconds.as_mut() else {
            return false;
        };
        *waited += dt_seconds.max(0.0);
        if *waited >= GUEST_WAITS_FOR_HOST_GAME_OVER_SECONDS {
            self.host_wait_seconds = None;
            return true;
        }
        false
    }

    pub fn on_host_game_over(&mut self) -> HostGameOverVerdict {
        if !self.checkpoint_taken {
            return HostGameOverVerdict::Drop(RewindDrop::NoCheckpoint);
        }
        if self.rewinding {
            return HostGameOverVerdict::Drop(RewindDrop::AlreadyRewinding);
        }
        self.rewinding = true;
        self.host_wait_seconds = None;
        HostGameOverVerdict::Rewind
    }

    pub fn finish_rewind(&mut self) {
        self.rewinding = false;
    }
}

impl HostGameOverGate {
    pub const fn new() -> Self {
        Self { armed_at_ms: None }
    }

    pub fn arm(&mut self, now_ms: u64) {
        self.armed_at_ms = Some(now_ms);
    }

    pub fn take(&mut self, now_ms: u64) -> bool {
        let armed = self.armed_at_ms.take();
        armed.is_some_and(|at| now_ms.saturating_sub(at) <= GAME_OVER_FLOW_WINDOW_MS)
    }
}

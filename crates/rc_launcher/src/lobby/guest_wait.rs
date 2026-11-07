use rc_proto::GameReadiness;

pub const GUEST_WAIT_FOR_HOST_GAME_SECONDS: f64 = 90.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestWaitStep {
    KeepWaiting,
    LaunchNow,
    GiveUp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuestWaiting {
    pub mode: u8,
    started_at: Option<f64>,
}

impl GuestWaiting {
    pub fn new(mode: u8) -> Self {
        Self {
            mode,
            started_at: None,
        }
    }

    pub fn step(&mut self, host_game: GameReadiness, now_seconds: f64) -> GuestWaitStep {
        let started_at = *self.started_at.get_or_insert(now_seconds);
        if host_game != GameReadiness::NotLinked {
            return GuestWaitStep::LaunchNow;
        }
        if now_seconds - started_at >= GUEST_WAIT_FOR_HOST_GAME_SECONDS {
            return GuestWaitStep::GiveUp;
        }
        GuestWaitStep::KeepWaiting
    }
}

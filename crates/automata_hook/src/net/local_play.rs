use std::sync::atomic::{AtomicBool, Ordering};

use automata_proto::GameReadiness;

use crate::net::READINESS_HEARTBEAT_MS;

static LOCAL_IN_PLAY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadinessPacer {
    sent: Option<(GameReadiness, u64)>,
}

impl ReadinessPacer {
    pub fn is_due(&self, now_ms: u64, current: GameReadiness) -> bool {
        match self.sent {
            None => true,
            Some((sent, _)) if sent != current => true,
            Some((_, at_ms)) => now_ms.saturating_sub(at_ms) >= READINESS_HEARTBEAT_MS,
        }
    }

    pub fn is_new(&self, current: GameReadiness) -> bool {
        self.sent.is_none_or(|(sent, _)| sent != current)
    }

    pub fn mark_sent(&mut self, now_ms: u64, current: GameReadiness) {
        self.sent = Some((current, now_ms));
    }

    pub fn forget(&mut self) {
        self.sent = None;
    }
}

pub fn set_local_in_play(in_play: bool) {
    LOCAL_IN_PLAY.store(in_play, Ordering::Release);
}

pub fn local_in_play() -> bool {
    LOCAL_IN_PLAY.load(Ordering::Acquire)
}

pub fn readiness_of(in_play: bool) -> GameReadiness {
    if in_play {
        GameReadiness::InPlay
    } else {
        GameReadiness::Loading
    }
}

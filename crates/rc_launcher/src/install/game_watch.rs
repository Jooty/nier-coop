use std::collections::BTreeSet;

pub const GAME_NEVER_APPEARED_SECONDS: f64 = 120.0;
pub const GAME_WATCH_SETTLE_SECONDS: f64 = 20.0;
pub const GAME_GONE_POLLS_BEFORE_EXIT: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameWatchStep {
    Waiting,
    Running,
    Exited,
    NeverAppeared,
}

#[derive(Debug, Clone)]
pub struct GameWatch {
    baseline: BTreeSet<u32>,
    ours: BTreeSet<u32>,
    started_at: Option<f64>,
    first_seen_at: Option<f64>,
    polls_gone: u32,
}

impl GameWatch {
    pub fn new(pids_before_launch: &[u32]) -> Self {
        Self {
            baseline: pids_before_launch.iter().copied().collect(),
            ours: BTreeSet::new(),
            started_at: None,
            first_seen_at: None,
            polls_gone: 0,
        }
    }

    pub fn step(&mut self, pids_now: &[u32], now_seconds: f64) -> GameWatchStep {
        let started_at = *self.started_at.get_or_insert(now_seconds);
        let settled = self
            .first_seen_at
            .is_some_and(|seen| now_seconds - seen >= GAME_WATCH_SETTLE_SECONDS);
        if !settled {
            self.ours.extend(
                pids_now
                    .iter()
                    .copied()
                    .filter(|pid| !self.baseline.contains(pid)),
            );
        }
        if self.ours.is_empty() {
            return if now_seconds - started_at >= GAME_NEVER_APPEARED_SECONDS {
                GameWatchStep::NeverAppeared
            } else {
                GameWatchStep::Waiting
            };
        }
        let first_seen_at = *self.first_seen_at.get_or_insert(now_seconds);
        if self.ours.iter().any(|pid| pids_now.contains(pid)) {
            self.polls_gone = 0;
            return GameWatchStep::Running;
        }
        self.polls_gone += 1;
        let settled_now = now_seconds - first_seen_at >= GAME_WATCH_SETTLE_SECONDS;
        if settled_now && self.polls_gone >= GAME_GONE_POLLS_BEFORE_EXIT {
            GameWatchStep::Exited
        } else {
            GameWatchStep::Running
        }
    }
}

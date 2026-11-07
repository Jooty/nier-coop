use crate::move_log::{MOVE_LOG_DURATION_MS, MOVE_LOG_INTERVAL_MS, MoveLogReason};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoveLogDue {
    pub reason: MoveLogReason,
    pub seconds_in: u64,
    pub frames: u32,
    pub frame_ms_average: f32,
    pub frame_ms_longest: u64,
}

#[derive(Debug, Default)]
pub struct MoveLogWindow {
    reason: Option<MoveLogReason>,
    started_ms: u64,
    last_logged_ms: Option<u64>,
    last_tick_ms: Option<u64>,
    frames: u32,
    frame_ms_total: u64,
    frame_ms_longest: u64,
}

impl MoveLogWindow {
    pub fn arm(&mut self, reason: MoveLogReason, now_ms: u64) {
        *self = Self {
            reason: Some(reason),
            started_ms: now_ms,
            last_tick_ms: Some(now_ms),
            ..Self::default()
        };
    }

    pub fn is_armed(&self) -> bool {
        self.reason.is_some()
    }

    pub fn on_tick(&mut self, now_ms: u64) -> Option<MoveLogDue> {
        let reason = self.reason?;
        let elapsed = now_ms.saturating_sub(self.started_ms);
        if elapsed > MOVE_LOG_DURATION_MS {
            self.reason = None;
            return None;
        }
        let frame_ms = now_ms.saturating_sub(self.last_tick_ms.unwrap_or(now_ms));
        self.last_tick_ms = Some(now_ms);
        self.frames += 1;
        self.frame_ms_total += frame_ms;
        self.frame_ms_longest = self.frame_ms_longest.max(frame_ms);
        let due = self
            .last_logged_ms
            .is_none_or(|logged| now_ms.saturating_sub(logged) >= MOVE_LOG_INTERVAL_MS);
        if !due {
            return None;
        }
        let report = MoveLogDue {
            reason,
            seconds_in: elapsed / 1000,
            frames: self.frames,
            frame_ms_average: self.frame_ms_total as f32 / self.frames as f32,
            frame_ms_longest: self.frame_ms_longest,
        };
        self.last_logged_ms = Some(now_ms);
        self.frames = 0;
        self.frame_ms_total = 0;
        self.frame_ms_longest = 0;
        Some(report)
    }
}

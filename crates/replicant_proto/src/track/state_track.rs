use std::collections::VecDeque;

use crate::track::{
    MAX_TRACK_SAMPLES, TRACK_HISTORY_MS, TRACK_RESTART_GAP_MS, TRACK_RESTART_TICK_JUMP, TrackSample,
};
use crate::wire::PlayerState;

#[derive(Clone, Debug, Default)]
pub struct StateTrack {
    samples: VecDeque<(u64, PlayerState)>,
}

impl StateTrack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, at_ms: u64, state: PlayerState) -> bool {
        if let Some((last_at, last)) = self.samples.back() {
            let tick_step = state.tick.wrapping_sub(last.tick).cast_signed();
            let restarted = at_ms.saturating_sub(*last_at) > TRACK_RESTART_GAP_MS
                || tick_step <= -TRACK_RESTART_TICK_JUMP;
            if restarted {
                self.samples.clear();
            } else if tick_step <= 0 {
                return false;
            }
        }
        let at_ms = self
            .samples
            .back()
            .map_or(at_ms, |(last_at, _)| at_ms.max(*last_at));
        self.samples.push_back((at_ms, state));
        let horizon = at_ms.saturating_sub(TRACK_HISTORY_MS);
        while self.samples.len() > 2 && self.samples[1].0 <= horizon {
            self.samples.pop_front();
        }
        while self.samples.len() > MAX_TRACK_SAMPLES {
            self.samples.pop_front();
        }
        true
    }

    pub fn sample(&self, now_ms: u64, delay_ms: u64) -> Option<TrackSample> {
        let target = now_ms.saturating_sub(delay_ms);
        let (first_at, first) = self.samples.front()?;
        if target < *first_at {
            return Some(TrackSample::of(first));
        }
        let later_index = self.samples.partition_point(|(at, _)| *at <= target);
        let (earlier_at, earlier) = &self.samples[later_index - 1];
        let elapsed_ms = target - earlier_at;
        let Some((later_at, later)) = self.samples.get(later_index) else {
            return Some(TrackSample::held(earlier, elapsed_ms));
        };
        let span = (later_at - earlier_at) as f32;
        let fraction = if span > 0.0 {
            (target - earlier_at) as f32 / span
        } else {
            1.0
        };
        Some(TrackSample::between(earlier, later, fraction, elapsed_ms))
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

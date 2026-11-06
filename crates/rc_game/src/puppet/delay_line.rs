use std::collections::VecDeque;

use crate::puppet::MAX_MIRROR_DELAY_MS;

#[derive(Debug, Clone)]
pub struct DelayLine<T> {
    samples: VecDeque<(u64, T)>,
}

impl<T> Default for DelayLine<T> {
    fn default() -> Self {
        Self {
            samples: VecDeque::new(),
        }
    }
}

impl<T> DelayLine<T> {
    pub fn push(&mut self, at_ms: u64, sample: T) {
        self.samples.push_back((at_ms, sample));
        let horizon = at_ms.saturating_sub(u64::from(MAX_MIRROR_DELAY_MS));
        while self.samples.len() >= 2 && self.samples[1].0 <= horizon {
            self.samples.pop_front();
        }
    }

    pub fn sample_delayed(&self, now_ms: u64, delay_ms: u64) -> Option<&T> {
        let wanted = now_ms.saturating_sub(delay_ms);
        self.samples
            .iter()
            .rev()
            .find(|(at_ms, _)| *at_ms <= wanted)
            .or_else(|| self.samples.front())
            .map(|(_, sample)| sample)
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

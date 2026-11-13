#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryVerdict {
    /// Make the call now; this is try number `n`, counted from 1.
    Try(u32),
    /// The last try was less than the retry interval ago.
    Wait,
    /// The last allowed try did not take: log the refusal once, and try no more until reset.
    GiveUp,
    /// Given up before; nothing is tried until reset.
    Refused,
}

/// A call retried at an interval, at most a number of times, then refused until reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoundedTries {
    tries: u32,
    last_try_ms: Option<u64>,
    refused: bool,
}

impl BoundedTries {
    pub fn next(&mut self, now_ms: u64, retry_ms: u64, most: u32) -> TryVerdict {
        if self.refused {
            return TryVerdict::Refused;
        }
        if self
            .last_try_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < retry_ms)
        {
            return TryVerdict::Wait;
        }
        if self.tries >= most {
            self.refused = true;
            return TryVerdict::GiveUp;
        }
        self.tries += 1;
        self.last_try_ms = Some(now_ms);
        TryVerdict::Try(self.tries)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn tries(&self) -> u32 {
        self.tries
    }
}

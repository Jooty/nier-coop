use serde::{Deserialize, Serialize};

/// What a state message says about the sender's loading screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LoadSignal {
    /// Sent from a loading screen or an arrival: the body fields of the message mean nothing.
    pub without_body: bool,
    /// The sender's load is done and it stays on its loading screen for the partner.
    pub waiting: bool,
    /// Milliseconds from the sending until the sender leaves its loading screen, on the sender's
    /// own clock; negative once it has left.
    pub release_in_ms: Option<i64>,
}

/// What this game says about its loading screen, with the release as a moment on this game's own clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalLoadSignal {
    pub without_body: bool,
    pub waiting: bool,
    pub release_at_ms: Option<u64>,
}

impl LocalLoadSignal {
    pub fn at(self, now_ms: u64) -> LoadSignal {
        LoadSignal {
            without_body: self.without_body,
            waiting: self.waiting,
            release_in_ms: self.release_at_ms.map(|release_at_ms| {
                let gap_ms = i128::from(release_at_ms) - i128::from(now_ms);
                i64::try_from(gap_ms).unwrap_or(if gap_ms < 0 { i64::MIN } else { i64::MAX })
            }),
        }
    }
}

use serde::{Deserialize, Serialize};

/// What a state message says about the sender's loading screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LoadSignal {
    /// Sent from a loading screen or an arrival: the body fields of the message mean nothing.
    pub without_body: bool,
    /// The sender's load is done and it stays on its loading screen for the partner.
    pub waiting: bool,
    /// The moment the sender leaves, or left, its loading screen, on the wall clock.
    pub release_at_unix_ms: Option<u64>,
}

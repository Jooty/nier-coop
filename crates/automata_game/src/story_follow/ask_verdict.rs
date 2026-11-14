use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AskVerdict {
    Made,
    AlreadyMade,
    RefusedOtherPlace,
    RefusedBusyTooLong,
    RefusedGuestWatches,
}

impl AskVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Made => "made",
            Self::AlreadyMade => "already made",
            Self::RefusedOtherPlace => "refused, the host is at another place",
            Self::RefusedBusyTooLong => "refused, the host was busy too long",
            Self::RefusedGuestWatches => "refused, the guest watches and never leads",
        }
    }

    pub fn ends_the_ask_with_the_request_done(self) -> bool {
        matches!(self, Self::Made | Self::AlreadyMade)
    }
}

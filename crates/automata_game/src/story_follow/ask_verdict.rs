use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AskVerdict {
    Made,
    AlreadyMade,
    RefusedOtherPlace,
    RefusedBusyTooLong,
}

impl AskVerdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Made => "made",
            Self::AlreadyMade => "already made",
            Self::RefusedOtherPlace => "refused, the host is at another place",
            Self::RefusedBusyTooLong => "refused, the host was busy too long",
        }
    }

    pub fn ends_the_ask_with_the_request_done(self) -> bool {
        matches!(self, Self::Made | Self::AlreadyMade)
    }
}

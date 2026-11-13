use serde::{Deserialize, Serialize};

use crate::event_follow::EventId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventAskWhat {
    Start,
    Skip,
}

/// The guest asks the host to start or to skip one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAsk {
    pub id: EventId,
    pub what: EventAskWhat,
}

impl EventAskWhat {
    pub fn label(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Skip => "skip",
        }
    }
}

impl EventAsk {
    pub fn is_valid(&self) -> bool {
        self.id.is_valid()
    }
}

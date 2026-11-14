use serde::{Deserialize, Serialize};

use crate::load_gate::StoryPlace;

/// The phase and sub-phase the restore point's placement record names, and whether it holds a
/// point of its own (else the phase's default start).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorePlace {
    pub place: StoryPlace,
    pub has_a_point: bool,
}

impl RestorePlace {
    pub fn is_valid(&self) -> bool {
        self.place.name_is_valid()
    }

    pub fn describe(&self) -> String {
        let start = if self.has_a_point {
            "at its saved point"
        } else {
            "at the phase's default start"
        };
        format!("{} {start}", self.place.describe())
    }
}

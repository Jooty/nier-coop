use serde::{Deserialize, Serialize};

use crate::load_gate::{FIRST_SYSTEM_PHASE_ID, STORY_PLACE_NAME_MOST_BYTES, StoryPlace};
use crate::story_follow::DEFAULT_FULL_REQUEST_PARAMS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryRequest {
    Full {
        id: i32,
        name: String,
        params: [i32; 6],
    },
    SubPhase {
        id: i32,
        name: String,
    },
}

impl StoryRequest {
    pub fn full_with_default_params(id: i32, name: &str) -> Self {
        Self::Full {
            id,
            name: name.to_owned(),
            params: DEFAULT_FULL_REQUEST_PARAMS,
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            Self::Full { id, .. } | Self::SubPhase { id, .. } => *id,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Full { name, .. } | Self::SubPhase { name, .. } => name,
        }
    }

    pub fn target_place(&self) -> StoryPlace {
        StoryPlace::new(self.id(), self.name())
    }

    pub fn is_valid(&self) -> bool {
        let name = self.name();
        let name_fits = name.len() <= STORY_PLACE_NAME_MOST_BYTES
            && name.bytes().all(|byte| byte.is_ascii_graphic());
        let named_when_it_must_be = matches!(self, Self::Full { .. }) || !name.is_empty();
        (0..FIRST_SYSTEM_PHASE_ID).contains(&self.id()) && name_fits && named_when_it_must_be
    }

    pub fn describe(&self) -> String {
        match self {
            Self::Full { id, name, params } => {
                format!("full {id:#X} {name} params {params:?}")
            }
            Self::SubPhase { id, name } => format!("sub-phase {id:#X} {name}"),
        }
    }
}

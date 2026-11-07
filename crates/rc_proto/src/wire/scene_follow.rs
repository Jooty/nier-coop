use serde::{Deserialize, Serialize};

use crate::wire::{MAX_STORY_CALL_NAME_BYTES, WireError};

pub const MAX_SCENE_FOLLOW_CALLBACKS: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFollow {
    pub phase: u16,
    pub callbacks: Vec<String>,
}

impl SceneFollow {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.callbacks.len() > MAX_SCENE_FOLLOW_CALLBACKS {
            return Err(WireError::OutOfRange {
                field: "scene follow callbacks",
            });
        }
        let bad_name = self.callbacks.iter().any(|name| {
            let bytes = name.as_bytes();
            bytes.is_empty()
                || bytes.len() > MAX_STORY_CALL_NAME_BYTES
                || !bytes.iter().all(u8::is_ascii_graphic)
        });
        if bad_name {
            return Err(WireError::OutOfRange {
                field: "scene follow callback name",
            });
        }
        Ok(())
    }
}

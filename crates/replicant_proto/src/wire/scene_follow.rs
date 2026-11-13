use serde::{Deserialize, Serialize};

use crate::wire::{MAX_STORY_CALL_NAME_BYTES, WireError};

pub const MAX_SCENE_FOLLOW_CALLBACKS: usize = 4;
pub const MAX_SCENE_CUT_NAME_BYTES: usize = 31;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFollow {
    pub phase: u16,
    pub callbacks: Vec<String>,
    pub cut: Option<SceneCut>,
}

/// The event cut the host's game had loaded when it entered the scene: the event name the
/// scene's data was loaded under, and the audio name its start asked for (empty when none).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneCut {
    pub event: String,
    pub audio: String,
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
        if let Some(cut) = &self.cut {
            cut.validate()?;
        }
        Ok(())
    }
}

impl SceneCut {
    pub fn validate(&self) -> Result<(), WireError> {
        let event_ok = !self.event.is_empty() && is_cut_name(&self.event);
        let audio_ok = is_cut_name(&self.audio);
        if event_ok && audio_ok {
            Ok(())
        } else {
            Err(WireError::OutOfRange {
                field: "scene follow cut name",
            })
        }
    }
}

fn is_cut_name(name: &str) -> bool {
    name.len() <= MAX_SCENE_CUT_NAME_BYTES && name.bytes().all(|byte| byte.is_ascii_graphic())
}

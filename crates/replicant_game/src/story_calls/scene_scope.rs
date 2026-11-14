use crate::callback_gate::GATED_CALLBACK_NAMES;
use crate::own_ride::is_the_riders_own;
use crate::story_calls::{
    LOCAL_SCENE_NAME_PARTS_LOWERCASE, LOCAL_SCENE_NAME_PREFIXES, LOCAL_SCENE_NAMES,
    is_event_callback_name, is_own_body_callback_name,
};

/// Who a scripted call or event callback belongs to when two players share a story.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneScope {
    SharedProgress,
    /// A talk, cutscene or fight opener that both players see when they stand in the same place.
    SharedScene,
    /// Doors, area exits, camera, light, ambient sound, self-talk, the player's own fall or
    /// respawn and the player's own ride: only the player it happened to runs it, and it is never
    /// relayed to or forced on the other game.
    LocalScene,
}

impl SceneScope {
    pub fn of(name: &str) -> Self {
        if is_own_body_callback_name(name) || is_the_riders_own(name) {
            return Self::LocalScene;
        }
        if is_event_callback_name(name) || GATED_CALLBACK_NAMES.contains(&name) {
            return Self::SharedScene;
        }
        let lowered = name.to_ascii_lowercase();
        let local = LOCAL_SCENE_NAMES.contains(&name)
            || LOCAL_SCENE_NAME_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
            || LOCAL_SCENE_NAME_PARTS_LOWERCASE
                .iter()
                .any(|part| lowered.contains(part));
        if local {
            Self::LocalScene
        } else {
            Self::SharedProgress
        }
    }

    pub fn is_local(self) -> bool {
        self == Self::LocalScene
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::SharedProgress => "shared progress",
            Self::SharedScene => "shared scene",
            Self::LocalScene => "local scene",
        }
    }
}

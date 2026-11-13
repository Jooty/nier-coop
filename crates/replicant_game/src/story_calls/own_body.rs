use crate::story_calls::{OWN_BODY_CALLBACK_NAME_PREFIXES, OWN_BODY_CALLBACK_NAMES};

/// A callback the game fires for the local player's own fall, drowning or respawn: it fades that
/// player's screen and moves that player's body. The story flags it writes still reach the
/// partner, through the story feed and the local scene watch.
pub fn is_own_body_callback_name(name: &str) -> bool {
    OWN_BODY_CALLBACK_NAMES.contains(&name)
        || OWN_BODY_CALLBACK_NAME_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

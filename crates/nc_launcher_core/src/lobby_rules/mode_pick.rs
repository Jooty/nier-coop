use nc_wire::SessionRules;

use crate::profile::{GameMode, GameProfile};

/// The rules the host's launcher sends when the host picks `mode_id`, or `None` when that mode
/// cannot be picked (unknown to the game or only planned).
pub fn rules_for_mode(profile: &GameProfile, mode_id: u8) -> Option<SessionRules> {
    profile
        .mode(mode_id)
        .filter(|mode| mode.pickable())
        .map(GameMode::rules)
}

/// The mode the session's rules name, falling back to the game's first mode.
pub fn current_mode(profile: &GameProfile, rules: SessionRules) -> Option<&'static GameMode> {
    profile.mode(rules.mode).or_else(|| profile.modes.first())
}

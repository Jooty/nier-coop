use nt_wire::SessionRules;

use crate::lobby_rules::current_mode;
use crate::profile::GameProfile;

/// Two players may wear one character where the game allows it, and in every mode but the campaign.
pub fn characters_shareable(profile: &GameProfile, rules: SessionRules) -> bool {
    profile.characters_shareable
        || current_mode(profile, rules).is_none_or(|mode| !mode.is_campaign())
}

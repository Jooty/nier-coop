use nc_wire::SessionRules;

use crate::lobby_rules::characters_shareable;
use crate::profile::{Character, GameProfile};

pub fn character_allowed(
    profile: &GameProfile,
    rules: SessionRules,
    wanted: &str,
    taken_by_others: &[&str],
) -> bool {
    profile.character(wanted).is_some()
        && (characters_shareable(profile, rules) || !taken_by_others.contains(&wanted))
}

pub fn character_choices(
    profile: &GameProfile,
    rules: SessionRules,
    taken_by_others: &[&str],
) -> Vec<&'static Character> {
    profile
        .characters
        .iter()
        .filter(|character| character_allowed(profile, rules, character.id, taken_by_others))
        .collect()
}

use crate::profile::{Character, GameProfile};

pub fn character_allowed(profile: &GameProfile, wanted: &str, taken_by_others: &[&str]) -> bool {
    profile.character(wanted).is_some()
        && (profile.characters_shareable || !taken_by_others.contains(&wanted))
}

pub fn character_choices(
    profile: &GameProfile,
    taken_by_others: &[&str],
) -> Vec<&'static Character> {
    profile
        .characters
        .iter()
        .filter(|character| character_allowed(profile, character.id, taken_by_others))
        .collect()
}

use nc_launcher_core::{Character, GameProfile, REPLICANT, character_allowed, character_choices};

pub fn lobby_picks_character_id(profile: &GameProfile) -> bool {
    profile.game_id != REPLICANT.game_id
}

pub fn host_character_id(profile: &GameProfile) -> &'static str {
    profile
        .characters
        .first()
        .map_or("", |character| character.id)
}

pub fn guest_character_choices(profile: &GameProfile) -> Vec<&'static Character> {
    character_choices(profile, &[host_character_id(profile)])
}

pub fn own_character_id(
    profile: &GameProfile,
    is_guest: bool,
    wanted: Option<&str>,
) -> &'static str {
    let host = host_character_id(profile);
    if !is_guest {
        return host;
    }
    wanted
        .filter(|id| character_allowed(profile, id, &[host]))
        .and_then(|id| profile.character(id))
        .or_else(|| guest_character_choices(profile).first().copied())
        .map_or(host, |character| character.id)
}

pub fn character_label_of(profile: &GameProfile, id: &str) -> &'static str {
    profile
        .character(id)
        .map_or("unknown", |character| character.label)
}

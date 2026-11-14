use nc_launcher_core::{
    Character, GameProfile, REPLICANT, SessionRules, campaign_character, character_choices,
    current_mode,
};

pub fn lobby_picks_character_id(profile: &GameProfile) -> bool {
    profile.game_id != REPLICANT.game_id
}

pub fn default_character_id(profile: &GameProfile) -> &'static str {
    profile
        .characters
        .first()
        .map_or("", |character| character.id)
}

/// Whether the lobby offers a character choice: never in the co-op campaign, where the host plays the
/// story's hero and the guest the companion.
pub fn character_picked_in_lobby(profile: &GameProfile, rules: SessionRules) -> bool {
    lobby_picks_character_id(profile) && !campaign_rules(profile, rules)
}

pub fn own_character_id(
    profile: &GameProfile,
    rules: SessionRules,
    is_guest: bool,
    wanted: Option<&str>,
) -> &'static str {
    if campaign_rules(profile, rules) {
        return campaign_character(profile, is_guest)
            .map_or_else(|| default_character_id(profile), |character| character.id);
    }
    wanted
        .and_then(|id| profile.character(id))
        .map_or_else(|| default_character_id(profile), |character| character.id)
}

pub fn partner_character_id(
    profile: &GameProfile,
    rules: SessionRules,
    is_guest: bool,
    partner: Option<&str>,
) -> Option<&'static str> {
    if campaign_rules(profile, rules) {
        return campaign_character(profile, !is_guest).map(|character| character.id);
    }
    let told = partner.and_then(|id| profile.character(id));
    if is_guest {
        // A host launcher that sends no character holds the first one.
        return Some(told.map_or_else(|| default_character_id(profile), |character| character.id));
    }
    told.map(|character| character.id)
}

pub fn own_choices(profile: &GameProfile, rules: SessionRules) -> Vec<&'static Character> {
    if campaign_rules(profile, rules) {
        return Vec::new();
    }
    character_choices(profile, rules, &[])
}

pub fn character_label_of(profile: &GameProfile, id: &str) -> &'static str {
    profile
        .character(id)
        .map_or("unknown", |character| character.label)
}

pub fn character_index_of(profile: &GameProfile, id: &str) -> Option<u8> {
    profile
        .characters
        .iter()
        .position(|character| character.id == id)
        .and_then(|index| u8::try_from(index).ok())
}

pub fn character_id_at(profile: &GameProfile, index: u8) -> Option<&'static str> {
    profile
        .characters
        .get(usize::from(index))
        .map(|character| character.id)
}

fn campaign_rules(profile: &GameProfile, rules: SessionRules) -> bool {
    current_mode(profile, rules).is_none_or(|mode| mode.is_campaign())
}

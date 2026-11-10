use nc_launcher_core::{Character, GameProfile, REPLICANT, character_allowed, character_choices};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterAssignment {
    pub host: &'static str,
    pub guest: &'static str,
    pub guest_was_bumped: bool,
}

pub fn lobby_picks_character_id(profile: &GameProfile) -> bool {
    profile.game_id != REPLICANT.game_id
}

pub fn default_character_id(profile: &GameProfile) -> &'static str {
    profile
        .characters
        .first()
        .map_or("", |character| character.id)
}

pub fn assign_characters(
    profile: &GameProfile,
    host_wanted: Option<&str>,
    guest_wanted: Option<&str>,
) -> CharacterAssignment {
    let host = host_wanted
        .and_then(|id| profile.character(id))
        .map_or_else(|| default_character_id(profile), |character| character.id);
    let guest_wish = guest_wanted.and_then(|id| profile.character(id));
    let guest_fits =
        guest_wish.filter(|character| character_allowed(profile, character.id, &[host]));
    let guest = guest_fits
        .or_else(|| character_choices(profile, &[host]).first().copied())
        .map_or(host, |character| character.id);
    CharacterAssignment {
        host,
        guest,
        guest_was_bumped: guest_wish.is_some() && guest_fits.is_none(),
    }
}

pub fn own_character_id(
    profile: &GameProfile,
    is_guest: bool,
    wanted: Option<&str>,
    partner: Option<&str>,
) -> &'static str {
    let assignment = if is_guest {
        let host = partner.unwrap_or_else(|| default_character_id(profile));
        assign_characters(profile, Some(host), wanted)
    } else {
        assign_characters(profile, wanted, partner)
    };
    if is_guest {
        assignment.guest
    } else {
        assignment.host
    }
}

pub fn partner_character_id(
    profile: &GameProfile,
    is_guest: bool,
    wanted: Option<&str>,
    partner: Option<&str>,
) -> Option<&'static str> {
    if is_guest {
        let host = partner.unwrap_or_else(|| default_character_id(profile));
        return Some(assign_characters(profile, Some(host), wanted).host);
    }
    partner.map(|_| assign_characters(profile, wanted, partner).guest)
}

pub fn own_choices(
    profile: &GameProfile,
    is_guest: bool,
    partner: Option<&str>,
) -> Vec<&'static Character> {
    let partner = if is_guest {
        Some(partner.unwrap_or_else(|| default_character_id(profile)))
    } else {
        partner
    };
    let taken: Vec<&str> = partner.into_iter().collect();
    character_choices(profile, &taken)
}

pub fn guest_bump_notice(
    profile: &GameProfile,
    wanted: Option<&str>,
    host: Option<&str>,
) -> Option<String> {
    let host_id = host.unwrap_or_else(|| default_character_id(profile));
    let assignment = assign_characters(profile, Some(host_id), wanted);
    assignment.guest_was_bumped.then(|| {
        format!(
            "{} is taken by the host, you play {}",
            character_label_of(profile, wanted.unwrap_or_default()),
            character_label_of(profile, assignment.guest)
        )
    })
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

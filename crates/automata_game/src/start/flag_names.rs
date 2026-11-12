use crate::character::Character;
use crate::start::ROUTE_FLAGS_BY_HASH;

pub fn hero_flag(character: Character) -> &'static str {
    match character {
        Character::TwoB => "PL/2B",
        Character::NineS => "PL/9S",
        Character::A2 => "PL/A2",
    }
}

pub fn buddy_flag(character: Character) -> &'static str {
    match character {
        Character::TwoB => "Buddy/2B",
        Character::NineS => "Buddy/9S",
        Character::A2 => "Buddy/A2",
    }
}

pub fn route_flag(route: i32, hero: Character) -> &'static str {
    match route {
        1 => "Route/B",
        3 => "Route/C",
        2 if hero == Character::NineS => "Route/B",
        _ => "Route/A",
    }
}

pub fn hero_mask_has(mask: u8, character: Character) -> bool {
    let bit = match character {
        Character::TwoB => 0,
        Character::NineS => 1,
        Character::A2 => 2,
    };
    mask & (1 << bit) != 0
}

pub fn route_flag_label(hash: u32) -> &'static str {
    ROUTE_FLAGS_BY_HASH
        .iter()
        .find(|(flag_hash, _)| *flag_hash == hash)
        .map_or("no route flag", |(_, name)| name)
}

/// The hash of the first route flag the game holds, or 0 when it holds none.
pub fn route_flag_hash_held(is_set: impl Fn(u32) -> bool) -> u32 {
    ROUTE_FLAGS_BY_HASH
        .iter()
        .map(|(hash, _)| *hash)
        .find(|hash| is_set(*hash))
        .unwrap_or(0)
}

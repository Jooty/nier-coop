use crate::profile::{AUTOMATA, GameProfile, REPLICANT};

pub const ALL_PROFILES: &[&GameProfile] = &[&REPLICANT, &AUTOMATA];

pub fn profile_by_id(game_id: &str) -> Option<&'static GameProfile> {
    ALL_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.game_id == game_id)
}

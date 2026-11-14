use crate::launcher_game::{GameLink, LauncherGame};
use crate::profile::{
    AUTOMATA, AUTOMATA_LAUNCHER_GAME, GameProfile, REPLICANT, REPLICANT_LAUNCHER_GAME,
};

pub const ALL_PROFILES: &[&GameProfile] = &[&REPLICANT, &AUTOMATA];
pub const ALL_GAMES: &[&dyn LauncherGame] = &[&REPLICANT_LAUNCHER_GAME, &AUTOMATA_LAUNCHER_GAME];

const UNREGISTERED_GAME_ID: &str = "";

#[derive(Debug)]
struct UnregisteredGame;

impl LauncherGame for UnregisteredGame {
    fn game_id(&self) -> &'static str {
        UNREGISTERED_GAME_ID
    }

    fn link(&self) -> &'static dyn GameLink {
        ALL_GAMES[0].link()
    }
}

pub fn profile_by_id(game_id: &str) -> Option<&'static GameProfile> {
    ALL_PROFILES
        .iter()
        .copied()
        .find(|profile| profile.game_id == game_id)
}

pub fn game_by_id(game_id: &str) -> &'static dyn LauncherGame {
    ALL_GAMES
        .iter()
        .copied()
        .find(|game| game.game_id() == game_id)
        .unwrap_or(&UnregisteredGame)
}

pub fn game_of(profile: &GameProfile) -> &'static dyn LauncherGame {
    game_by_id(profile.game_id)
}

pub const fn default_profile() -> &'static GameProfile {
    ALL_PROFILES[0]
}

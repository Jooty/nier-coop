mod automata;
mod character_art;
mod characters;
mod free_roam_map;
mod game_mode;
mod game_profile;
mod registry;
mod replicant;

pub use automata::{AUTOMATA, AUTOMATA_LAUNCHER_GAME};
pub use character_art::CharacterArt;
pub use characters::{character_allowed, character_choices};
pub use free_roam_map::FreeRoamMap;
pub use game_mode::{
    CAMPAIGN_PLAYER_COUNT, CO_OP_CAMPAIGN, FREE_ROAM, FREE_ROAM_MODE, GameMode,
};
pub use game_profile::{Character, GameProfile, GameSupport};
pub use registry::{ALL_GAMES, ALL_PROFILES, default_profile, game_by_id, game_of, profile_by_id};
pub use replicant::{REPLICANT, REPLICANT_LAUNCHER_GAME};

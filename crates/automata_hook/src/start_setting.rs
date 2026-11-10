mod game_settings;

pub use game_settings::{GameSettings, SettingsEnv, choose_game_settings, settings_from_launcher};

pub const PARTNER_CHARACTER_ENV: &str = "NIER_COOP_PARTNER_CHARACTER";
pub const START_ENV: &str = "NIER_COOP_START";
pub const START_INDEX_ENV: &str = "NIER_COOP_START_INDEX";
pub const START_SLOT_ENV: &str = "NIER_COOP_START_SLOT";
pub const PARTNER_DRIVE_ENV: &str = "NIER_COOP_PARTNER_DRIVE";
pub const STORY_ENV: &str = "NIER_COOP_STORY";

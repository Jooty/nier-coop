mod file;
mod start_choice;

pub use file::{
    body_text, clear_boot, clear_link, write_boot, write_character, write_dev_switches, write_link,
    write_log, write_new_game, write_settings, write_start,
};
pub use start_choice::{START_CHOICES, StartChoice, StartSetting};

pub const LINK_KEY: &str = "link";
pub const CHARACTER_KEY: &str = "character";
pub const PARTNER_CHARACTER_KEY: &str = "partner_character";
pub const START_KEY: &str = "start";
pub const START_INDEX_KEY: &str = "start_index";
pub const START_SLOT_KEY: &str = "start_slot";
pub const STORY_KEY: &str = "story";
pub const ROLE_KEY: &str = "role";
pub const BODY_KEY: &str = "body";
pub const NAME_KEY: &str = "name";
pub const BOOT_SLOT_KEY: &str = "boot_slot";
pub const NEW_GAME_SLOT_KEY: &str = "new_game_slot";
pub const NEW_GAME_NAME_KEY: &str = "new_game_name";
pub const BLOCK_SAVES_KEY: &str = "block_saves";
pub const KEEP_RUNNING_KEY: &str = "keep_running";
pub const DEV_NO_FOCUS_MENU_KEY: &str = "dev_no_focus_menu";
pub const SHOW_DISTANCE_KEY: &str = "show_distance";
pub const SHOW_PING_KEY: &str = "show_ping";
pub const EVENT_LOG_KEY: &str = "event_log";
pub const SHOW_NAMES_KEY: &str = "show_names";
pub const SHOW_PARTY_FRAME_KEY: &str = "show_party_frame";
pub const LOG_KEY: &str = "log";
pub const ENEMY_HEALTH_PERCENT_KEY: &str = "enemy_health_percent";
pub const ENEMY_DAMAGE_PERCENT_KEY: &str = "enemy_damage_percent";

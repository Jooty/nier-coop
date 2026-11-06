mod backup;
mod coop;
mod fresh;
mod location;
mod restore;
mod slots;

pub use backup::{BackupError, backup_saves, copy_dir_recursive, format_stamp};
pub use coop::{
    COOP_BACKUP_FOLDER, COOP_BEFORE_RESTORE_PREFIX, CoopTargetError, backup_coop_file,
    coop_copy_target_slot, coop_gamedata_path, copy_slot_into_coop, first_empty_slot, merge_slot,
    read_coop_slots, restore_coop_backup,
};
pub use fresh::fresh_coop_file;
pub use location::{backup_root, coop_backup_root, save_dir};
pub use rc_game::save_files::GAMEDATA_FILE_NAME;
pub use restore::{PRE_RESTORE_PREFIX, list_backups, restore_backup};
pub use slots::{
    SlotError, SlotSummary, default_slot, gamedata_path, parse_slots, read_slots, slot_player_name,
};

pub const MY_GAMES_FOLDER: &str = "My Games";
pub const BACKUP_FOLDER: &str = "save-backups";
pub const PROTON_SAVE_PREFIX_SEGMENTS: [&str; 3] = ["drive_c", "users", "steamuser"];
pub const STEAM_SAVE_FOLDER: &str = "Steam";
pub const GAMEDATA_SIZE: usize = 0x48200;
pub const SLOT_COUNT: u8 = 7;
pub const SLOTS_START: usize = 0x8160;
pub const SLOT_STRIDE: usize = 0x9260;
pub const HEADER_SLOT_MASK_OFFSET: usize = 0x6C;
pub const HEADER_ENDING_RECORD_RANGES: [std::ops::Range<usize>; 3] =
    [0x04..0x08, 0x50..0x57, 0x176..0x1A5];
pub const SLOT_VERSIONS: [u32; 2] = [200, 0x84];
pub const SLOT_VERSION_OFFSET: usize = 0x00;
pub const SLOT_PHASE_OFFSET: usize = 0x04;
pub const SLOT_PHASE_LENGTH: usize = 0x20;
pub const SLOT_NAME_OFFSET: usize = 0x2C;
pub const SLOT_NAME_LENGTH: usize = 0x20;
pub const SLOT_LEVEL_OFFSET: usize = 0x64;
pub const SLOT_PLAYTIME_OFFSET: usize = 0x4A0;

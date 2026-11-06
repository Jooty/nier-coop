mod backup;
mod location;

pub use backup::{BackupError, backup_saves, copy_dir_recursive, format_stamp};
pub use location::{backup_root, save_dir};

pub const MY_GAMES_FOLDER: &str = "My Games";
pub const BACKUP_FOLDER: &str = "save-backups";
pub const PROTON_SAVE_PREFIX_SEGMENTS: [&str; 3] = ["drive_c", "users", "steamuser"];

mod command;
mod launch;

pub use command::{CopyCommand, copy_command, copy_environment, proton_script_from_config_info};
pub use launch::start_second_copy;

pub const LINK_ENV: &str = "REPLICANT_COOP_LINK";
pub const NAME_ENV: &str = "REPLICANT_COOP_NAME";
pub const BOOT_SLOT_ENV: &str = "REPLICANT_COOP_BOOT_SLOT";
pub const BLOCK_SAVES_ENV: &str = "REPLICANT_COOP_BLOCK_SAVES";
pub const PROTON_SCRIPT_NAME: &str = "proton";

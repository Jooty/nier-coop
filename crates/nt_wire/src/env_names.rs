mod env_read;
mod old_name;

pub use env_read::{
    EnvRead, read_env_noting_old_name, read_env_os_noting_old_name, read_env_with_old_name,
};
pub use old_name::{old_env_name, old_name_note};

pub const ENV_PREFIX: &str = "NIERTOGETHER_";
pub const OLD_ENV_PREFIX: &str = "NIER_COOP_";

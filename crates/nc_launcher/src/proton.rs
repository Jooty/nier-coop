mod overrides;
mod prefix;

pub use overrides::{OverrideError, add_override, override_is_set, override_key, remove_override};
pub use prefix::{environ_uses_prefix, find_proton_files_dir, prefix_is_busy};

pub const DLL_OVERRIDE_VALUE: &str = "native,builtin";
pub const USER_REG_BACKUP_SUFFIX: &str = "replicant-coop.bak";

use std::sync::OnceLock;

use crate::net::AutoConfig;

static HARNESS_ENABLED: OnceLock<bool> = OnceLock::new();

pub fn injection_enabled() -> bool {
    *HARNESS_ENABLED.get_or_init(read_harness_flag)
}

fn read_harness_flag() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
        .and_then(|folder| AutoConfig::load_from_folder(&folder).ok().flatten())
        .is_some_and(|config| config.harness)
}

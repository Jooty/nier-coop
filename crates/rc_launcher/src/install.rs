mod files;
mod hashing;
mod ledger;
mod running;
mod source;

pub use files::{InstallError, InstallOutcome, ModStatus, install_dll, mod_status, uninstall_dll};
pub use hashing::sha256_file;
pub use ledger::Ledger;
pub use running::{cmdline_runs_exe, game_running, open_url};
pub use source::{dll_next_to_executable, fetch_dll_from_ci, newer_dll, newest_cached_ci_dll};

pub const PROXY_DLL_STEM: &str = "xinput9_1_0";
pub const PROXY_DLL_NAME: &str = "xinput9_1_0.dll";
pub const HOOK_DLL_NAME: &str = "rc_hook.dll";
pub const LEDGER_FILE_NAME: &str = "installed.sha256";

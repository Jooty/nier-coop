mod files;
mod game_watch;
mod hashing;
mod ledger;
mod mismatch;
mod running;
mod source;

pub use files::{InstallError, InstallOutcome, ModStatus, install_dll, mod_status, uninstall_dll};
pub use game_watch::{
    GAME_GONE_POLLS_BEFORE_EXIT, GAME_NEVER_APPEARED_SECONDS, GAME_WATCH_SETTLE_SECONDS, GameWatch,
    GameWatchStep,
};
pub use hashing::sha256_file;
pub use ledger::Ledger;
pub use mismatch::mod_mismatch;
pub use running::{
    cmdline_runs_exe, end_processes, game_pids, game_running, open_url, pids_from_tasklist_csv,
};
pub use source::{dll_next_to_executable, fetch_dll_from_ci, newer_dll, newest_cached_ci_dll};

pub const PROXY_DLL_STEM: &str = "xinput9_1_0";
pub const PROXY_DLL_NAME: &str = "xinput9_1_0.dll";
pub const HOOK_DLL_NAME: &str = "rc_hook.dll";
pub const LEDGER_FILE_NAME: &str = "installed.sha256";

mod detect;
mod discovery;
mod library;
mod vdf;

pub use detect::{detect_game, hide_console_window, read_registry_steam_path};
pub use discovery::{
    GameInstall, candidate_steam_roots, find_game, install_from_game_dir, parse_reg_query_value,
};
pub use library::{library_paths, manifest_install_dir};
pub use vdf::{VdfError, VdfNode, parse_vdf};

pub const APP_ID: &str = "1113560";
pub const EXE_NAME: &str = "NieR Replicant ver.1.22474487139.exe";
pub const GAME_FOLDER_NAME: &str = "NieR Replicant ver.1.22474487139";

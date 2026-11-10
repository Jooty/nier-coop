mod redirect_save_path;
mod save_file_kind;
mod save_redirect_status;
mod save_role;

pub use redirect_save_path::{SaveRedirect, redirect_save_path};
pub use save_file_kind::SaveFileKind;
pub use save_redirect_status::SaveRedirectStatus;
pub use save_role::SaveRole;

pub const SAVE_PATH_BUFFER_LEN: usize = 0x104;
pub const NORMAL_SAVE_FOLDER_SEGMENT: &str = "\\My Games\\NieR_Automata\\";
pub const COOP_SAVE_FOLDER_SEGMENT: &str = "\\My Games\\NieR_Automata_coop\\";

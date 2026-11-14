mod bundle;
mod endpoint;
mod gzip;
mod name;
mod select;
mod sent_marker;

pub use bundle::{BundleFile, BundleHeader, build_bundle};
pub use endpoint::{log_upload_offered, origin_of, upload_url};
pub use gzip::{gunzip_text, gzip_text};
pub use name::{is_allowed_upload_name, is_valid_install_id, new_install_id, upload_file_name};
pub use select::{LogFileEntry, is_run_log_name, pick_since, pick_unsent, role_of_files};
pub use sent_marker::{parse_sent_marker, render_sent_marker};

pub const LOG_FILE_TAIL_CAP_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_UPLOAD_BODY_BYTES: usize = 16 * 1024 * 1024;
pub const UPLOAD_TOKEN_HEADER: &str = "X-Upload-Token";
pub const UPLOAD_TIMEOUT_SECONDS: u64 = 30;
pub const UPLOAD_PATH_PREFIX: &str = "logs";
pub const UPLOAD_NAME_SUFFIX: &str = ".log.gz";
pub const MAX_UPLOAD_NAME_STEM_LENGTH: usize = 48;
pub const INSTALL_ID_LENGTH: usize = 32;
pub const BUNDLE_TITLE_LINE: &str = "nier-coop log bundle";
pub const NO_ROLE_NAME: &str = "launcher";

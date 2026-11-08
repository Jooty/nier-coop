mod apply;
mod channel;
mod check;
mod decision;
mod error;
mod fetch;
mod files;
mod manifest;
mod signature;

pub use apply::{apply_hook_update, apply_update, restore_hook_dll, swap_in_hook_dll};
pub use channel::{
    BuildConfig, BuildInputs, Channel, build_config, channel_label, own_channel_label,
    resolve_build_config,
};
pub use check::{AvailableUpdate, check_for_update};
pub use decision::{OwnBuild, is_newer, own_build, short_commit};
pub use error::UpdateError;
pub use fetch::{base_url_of, http_get};
pub use files::{download_verified, verify_file, with_query};
pub use manifest::{FileKind, Manifest, ManifestFile, UpdatePlan, parse_manifest, pick_files};
pub use signature::verify_manifest_signature;

pub const DEV_UPDATE_URL_FROM_BUILD: Option<&str> = option_env!("REPLICANT_COOP_UPDATE_URL");
pub const DEV_LOG_TOKEN_FROM_BUILD: Option<&str> = option_env!("REPLICANT_COOP_LOG_TOKEN");
pub const CHANNEL_FROM_BUILD: Option<&str> = option_env!("REPLICANT_COOP_CHANNEL");
pub const VERSION_FROM_BUILD: Option<&str> = option_env!("REPLICANT_COOP_VERSION");
pub const RELEASE_PUBLIC_KEY_HEX_FROM_BUILD: Option<&str> =
    option_env!("REPLICANT_COOP_UPDATE_PUBLIC_KEY_HEX");
pub const RELEASE_CHANNEL_NAME: &str = "release";
pub const RELEASE_UPDATE_URL: &str =
    "https://github.com/Jooty/replicant-coop/releases/latest/download/manifest.json";
pub const UPDATE_PUBLIC_KEY_HEX: &str =
    "f23861e82877ff5069ca9b864a32238624f05db56067a45e148f3a586be45cad";
pub const MANIFEST_SIGNATURE_SUFFIX: &str = ".sig";
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
pub const HTTP_TIMEOUT_SECONDS: u64 = 60;
pub const BUILD_COMMIT: &str = env!("RC_BUILD_COMMIT");
pub const BUILD_COMMIT_TIME_TEXT: &str = env!("RC_BUILD_COMMIT_TIME");
pub const SHORT_COMMIT_LENGTH: usize = 7;
pub const STAGING_DIR_NAME: &str = "rc_update_staging";
pub const OLD_HOOK_DLL_SUFFIX: &str = ".old";

mod bridge;
mod config;
mod core;
mod error;
mod event;
mod launcher_net;
mod log_upload;
mod probe;
mod profile;
mod report;

use std::time::Duration;

pub use config::{LauncherConfig, TransportFactory, iroh_factory};
pub use core::LauncherCore;
pub use error::NetError;
pub use event::{
    ConnectionState, LauncherEvent, LinkStatus, NetState, NetStatus, options_differ_from_last,
};
pub use launcher_net::LauncherNet;
pub use log_upload::{
    BUNDLE_TITLE_LINE, BundleFile, BundleHeader, LOG_FILE_TAIL_CAP_BYTES, LogFileEntry,
    MAX_UPLOAD_BODY_BYTES, UPLOAD_TIMEOUT_SECONDS, UPLOAD_TOKEN_HEADER, build_bundle, gunzip_text,
    gzip_text, is_allowed_upload_name, is_run_log_name, is_valid_install_id, log_upload_offered,
    new_install_id, origin_of, parse_sent_marker, pick_since, pick_unsent, render_sent_marker,
    role_of_files, upload_file_name, upload_url,
};
pub use profile::{
    ALL_PROFILES, Character, GameProfile, GameSupport, REPLICANT, character_allowed,
    character_choices, profile_by_id,
};
pub use replicant_proto::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, LOBBY_MODE_CONTINUE,
    LOBBY_MODE_NEW_GAME, LobbyReady, MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT,
    MAX_LOBBY_SLOT, PlayerSettings, SessionOptions,
};
pub use report::{
    ConnectionReport, PathKind, Quality, RttStats, Verdict, judge, judge_rejection_text,
};

pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
pub const PATH_INFO_INTERVAL: Duration = Duration::from_millis(500);
pub const PROBE_SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
pub const PROBE_MIN_SECONDS: u32 = 2;
pub const PROBE_MAX_SECONDS: u32 = 120;
pub const COMMAND_REPLY_TIMEOUT: Duration = Duration::from_secs(40);
pub const DEFAULT_PLAYER_NAME: &str = "Nier";

pub const INVALID_TICKET_TEXT: &str = "that ticket is not valid";

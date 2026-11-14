mod banner_art;
mod boot_console;
mod bridge;
mod campaign_flow;
mod config;
mod coop_save_delete;
mod copy_feedback;
mod core;
mod error;
mod event;
mod launcher_game;
mod launcher_net;
mod launcher_scale;
mod lobby_rules;
mod log_upload;
mod motion;
mod probe;
mod profile;
mod report;
mod seat_transfer;

use std::time::Duration;

use nc_wire::STORY_MODE;

pub use nc_wire::SessionRules;

pub(crate) const STORY_RULES: SessionRules = SessionRules {
    mode: STORY_MODE,
    max_players: CAMPAIGN_PLAYER_COUNT,
    map: 0,
};

pub use banner_art::{
    ArtCrop, ArtFocus, BannerArtSource, BannerFacts, FadeStop, SeatArt, SeatPicture, StripArt,
    WHOLE_PICTURE_STRIP, banner_facts, crop_around_focus, crop_share_around_focus, fade_alpha,
    fade_stops, find_banner_art, grey_dimmed, library_cache_hero_paths, seat_pictures, strip_art,
};
pub use boot_console::{
    CONSOLE_LINE_TYPING_MS, CONSOLE_SHORTEST_RUN_MS, ConsoleClock, ConsoleDetail, ConsoleEvent,
    ConsoleGameFacts, ConsoleLineKind, ConsoleLineView, ConsoleSave, ConsoleScript, ConsoleStep,
    ConsoleTask, ConsoleView, HostingFacts, JoiningFacts, StepEnd, TimedProgress, console_events,
    hosting_script, joining_script,
};
pub use bridge::members::{Members, RemoteMember};
pub use bridge::{protocol_version_of, version_difference_text};
pub use campaign_flow::{
    GuestSeatSync, GuestStartPlan, HostStartGate, HostStore, MAX_CAMPAIGN_ID_CHARACTERS,
    MIN_CAMPAIGN_ID_CHARACTERS, SEAT_DELIVERY_TIMEOUT_SECONDS, SESSION_FILE_ID_CHARACTERS,
    SESSION_FILE_PREFIX, SeatCandidate, SeatSource, StartGateStep, campaign_file_stem,
    host_store_decision, pick_newest_seat, plan_guest_start, seat_error_words, seat_refusal_words,
    session_file_name,
};
pub use config::{LauncherConfig, TransportFactory, iroh_factory};
pub use coop_save_delete::{
    CoopSaveDeleteError, CoopSaveDeleteOffer, DELETED_SAVES_FOLDER_SUFFIX,
    DELETED_SAVES_FOLDERS_KEPT, DeletedCoopSaves, coop_save_delete_offer, coop_saves_in,
    coop_saves_in_whole_folder, delete_coop_saves, is_coop_save_name, keep_newest_deleted_folders,
    whole_coop_saves_folder,
};
pub use copy_feedback::{COPIED_HOLD_SECONDS, copied_shown};
pub use core::LauncherCore;
pub use error::NetError;
pub use event::{
    ConnectionState, LauncherEvent, LinkStatus, MemberChange, MemberStatus, NetState, NetStatus,
    options_differ_from_last,
};
pub use launcher_game::{
    GameSaves, LauncherGame, SeatError, changed_slot, check_seat, coop_file_name,
    fix_header_checksum, is_coop_session_file_name, save_game, seat_bytes_of, seat_slot_bytes,
    session_file_with_seat, vanilla_file_name,
};
pub use launcher_net::LauncherNet;
pub use launcher_scale::LauncherScale;
pub use lobby_rules::{
    AfterSessionEnd, LobbyOption, LobbyPermissions, PlayerStatus, ROSTER_COLUMN_SEATS, ReadyInputs,
    Roster, RosterSeat, SeatNameChoice, SeatNameInputs, SessionEndCause, SessionEndSide,
    StartInputs, after_session_end, campaign_character, characters_shareable, current_map,
    current_mode, followed_lobby_kind, hosting_kind, lobby_permissions, lobby_ready,
    map_banner_path, map_pick_offered, option_available, own_seat_name, player_status,
    remove_offered, roster, rules_for_mode, rules_with_map, session_end_log_line, start_allowed,
    ticket_check,
};
pub use log_upload::{
    BUNDLE_TITLE_LINE, BundleFile, BundleHeader, LOG_FILE_TAIL_CAP_BYTES, LogFileEntry,
    MAX_UPLOAD_BODY_BYTES, UPLOAD_TIMEOUT_SECONDS, UPLOAD_TOKEN_HEADER, build_bundle, gunzip_text,
    gzip_text, is_allowed_upload_name, is_run_log_name, is_valid_install_id, log_upload_offered,
    new_install_id, origin_of, parse_sent_marker, pick_since, pick_unsent, render_sent_marker,
    role_of_files, upload_file_name, upload_url,
};
pub use motion::{
    ART_FADE_MS, Easing, HATCH_DRIFT_MS_PER_PX, METER_FOLLOW_MS, MotionClock, ROW_ART_BRIGHTEN_MS,
    SEAT_ENTER_MS, SEAT_FIRST_DELAY_MS, SEAT_STAGGER_MS, SECTION_ENTER_MS, SECTION_STAGGER_MS,
    SELECTION_SWEEP_MS, STATUS_INVERT_MS, Sweep, Transition, drift_offset_px,
};
pub use profile::{
    ALL_PROFILES, AUTOMATA, CAMPAIGN_PLAYER_COUNT, CO_OP_CAMPAIGN, Character, CharacterArt,
    FREE_ROAM, FREE_ROAM_MODE, FreeRoamMap, GameMode, GameProfile, GameSupport, HIDE_AND_SEEK,
    HIDE_AND_SEEK_MODE, ModeAvailability, PROP_HUNT, PROP_HUNT_MODE, REPLICANT, character_allowed,
    character_choices, default_profile, game_by_id, game_of, profile_by_id,
};
pub use replicant_proto::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, LOBBY_MODE_CONTINUE,
    LOBBY_MODE_NEW_GAME, LobbyReady, MAX_ENEMY_DAMAGE_PERCENT, MAX_ENEMY_HEALTH_PERCENT,
    MAX_LOBBY_SLOT, PlayerSettings, SessionOptions,
};
pub use report::{
    ConnectionReport, HOST_KEY_NOT_FOUND_TEXT, HOST_UNREACHABLE_TEXT, PathKind, Quality, RttStats,
    Verdict, judge, judge_rejection_text,
};
pub use seat_transfer::{
    AssemblerStep, SEAT_PARTS_PER_PUMP, SeatAssembler, SeatSender, seat_checksum,
};

pub const WORKER_WAKE_INTERVAL: Duration = Duration::from_millis(4);
pub const PATH_INFO_INTERVAL: Duration = Duration::from_millis(500);
pub const PROBE_SAMPLE_INTERVAL: Duration = Duration::from_millis(200);
pub const PROBE_MIN_SECONDS: u32 = 2;
pub const PROBE_MAX_SECONDS: u32 = 120;
pub const COMMAND_REPLY_TIMEOUT: Duration = Duration::from_secs(40);
pub const HOST_ANSWER_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_PLAYER_NAME: &str = "Nier";

pub const INVALID_TICKET_TEXT: &str = "that ticket is not valid";

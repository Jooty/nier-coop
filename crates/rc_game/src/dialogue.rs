//! The game's talk window: which talk and line a player is reading, and mirroring the partner's.
mod mirror;
mod talk_cursor;
mod talker_name;
mod tracker;
mod watcher_hold;

pub use mirror::{
    MirrorAction, MirrorInput, PartnerTalkMemory, PartnerTalkMirror, TalkLeader, talk_tags_run_here, within_talk_range,
};
pub use talk_cursor::{TalkCursor, find_talk_id, read_talk_cursor};
pub use talker_name::{replace_player_name_tag, speaker_name_to_show};
pub use tracker::DialogueTracker;
pub use watcher_hold::{WatcherHold, WatcherHoldAction, WatcherHoldInput, own_talk_started};

pub const TALK_WINDOW_RVA: u32 = 0x4C5_6970;
pub const TALK_WINDOW_OPEN_RVA: u32 = 0x4D_FF00;
pub const TALK_WINDOW_IS_BUSY_RVA: u32 = 0x4D_FAE0;
pub const TALK_WINDOW_UPDATE_RVA: u32 = 0x4E_0AF0;
pub const TALK_WINDOW_CLOSE_RVA: u32 = 0x4E_0A50;
pub const TALK_WINDOW_STATE_TABLE_RVA: u32 = 0x12F_3898;
pub const TALK_WINDOW_STATE_COUNT: i32 = 4;
pub const TALK_PLAYER_NEXT_LINE_RVA: u32 = 0x4E_3530;
pub const TALK_WINDOW_PLAYER_OFFSET: u64 = 0x108;
pub const TALK_WINDOW_CURRENT_TALK_OFFSET: u64 = 0x46_3040;
pub const TALK_WINDOW_CURRENT_LINE_OFFSET: u64 = 0x46_3048;
pub const TALK_WINDOW_MASK_BEFORE_OPEN_OFFSET: u64 = 0x4A_4170;
pub const TALK_WINDOW_TALK_TABLE_OFFSET: u64 = 0x1C_2B18;
pub const TALK_TABLE_SLOTS: u32 = 0x4_0000;
pub const TALK_TABLE_SCAN_CHUNK_SLOTS: u32 = 0x200;
pub const TALK_UPDATE_INPUT_BYTES: usize = 0x48;
pub const TALK_UPDATE_INPUT_ENABLED_OFFSET: usize = 0x42;
pub const GAME_STATE_RVA: u32 = 0x434_8610;
pub const CONTROL_MASK_SET_SLOT_ZERO_RVA: u32 = 0x3B_1370;
pub const CONTROL_MASK_TALK_SLOT_RVA: u32 = 0x435_AF88;
pub const TALK_TAG_RUN_ALL_RVA: u32 = 0xD_8950;
pub const MIRROR_OPEN_RETRY_MS: u64 = 1000;
pub const FOLLOW_HOST_TALK_WAIT_MS: u64 = 2000;
pub const OWN_ENDED_TALK_NOT_MIRRORED_MS: u64 = 3000;
pub const UI_DRAW_PASS_RVA: u32 = 0xC_C8F0;
pub const TALK_JOIN_RANGE_UNITS: f32 = 15.0;
pub const TALK_LEAVE_EXTRA_UNITS: f32 = 2.0;
pub const CONTROL_MASK_SET_SLOT_ONE_RVA: u32 = 0x3B_1950;
pub const WATCHER_HOLD_MASK: u32 = 0xA0;
pub const TALK_EXPAND_TAGS_RVA: u32 = 0xD_AB50;
pub const TALK_SPEAKER_NAME_RVA: u32 = 0x4E_2D90;
pub const PLAYER_NAME_TAG: &str = "<NIER>";

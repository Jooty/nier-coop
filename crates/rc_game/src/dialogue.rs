//! The game's talk window: which talk and line a player is reading, and mirroring the partner's.
mod mirror;
mod talk_cursor;
mod tracker;

pub use mirror::{MirrorAction, MirrorInput, PartnerTalkMirror};
pub use talk_cursor::{TalkCursor, find_talk_id, read_talk_cursor};
pub use tracker::DialogueTracker;

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
pub const MIRROR_OPEN_RETRY_MS: u64 = 1000;

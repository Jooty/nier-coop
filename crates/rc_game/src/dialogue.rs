//! The game's talk window: which talk and line a player is reading, and the line's text.
mod line_text;
mod talk_cursor;
mod tracker;

pub use line_text::{LineText, partner_line_events, read_line_text};
pub use talk_cursor::{TalkCursor, find_talk_id, read_talk_cursor};
pub use tracker::DialogueTracker;

pub const TALK_WINDOW_RVA: u32 = 0x4C5_6970;
pub const TALK_WINDOW_CURRENT_TALK_OFFSET: u64 = 0x46_3040;
pub const TALK_WINDOW_CURRENT_LINE_OFFSET: u64 = 0x46_3048;
pub const TALK_WINDOW_TALK_TABLE_OFFSET: u64 = 0x1C_2B18;
pub const TALK_TABLE_SLOTS: u32 = 0x4_0000;
pub const TALK_TABLE_SCAN_CHUNK_SLOTS: u32 = 0x200;
pub const TALK_LINE_COUNT_OFFSET: u64 = 0x28;
pub const TALK_LINES_OFFSET: u64 = 0x30;
pub const TALK_LINE_SPEAKER_ID_OFFSET: u64 = 0x2C;
pub const TALK_LINE_TEXT_OFFSET: u64 = 0x38;
pub const TALKER_NAME_TABLE_RVA: u32 = 0x27D_3570;
pub const TALKER_NAME_ENTRIES_OFFSET: u64 = 0xC010;
pub const TALKER_NAME_COUNT_OFFSET: u64 = 0xD010;
pub const TALKER_NAME_ENTRY_BYTES: u64 = 0x10;
pub const TALKER_NAME_MAX_ENTRIES: i32 = 0x100;
pub const MAX_TALK_LINES: i32 = 0x1000;
pub const MAX_TALK_TEXT_BYTES: usize = 0x1000;
pub const MAX_TALKER_NAME_BYTES: usize = 0x40;
pub const PROTAGONIST_NAME_TAG: &str = "<NIER>";

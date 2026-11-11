mod game_log_text;
mod lines;
mod overlay_log;

pub use game_log_text::{game_log_text_id, game_log_text_slot, game_log_text_units};
pub use lines::{CoOpLines, PlayerSeen, SessionSeen};
pub use overlay_log::{OverlayLog, OverlayLogLayout, VisibleLine, overlay_log_layout};

pub const PLACE_CHANGE_HOLDS_FOR_MS: u64 = 2_000;
pub const GAME_LOG_TEXT_SLOT_COUNT: usize = 16;
pub const FIRST_GAME_LOG_TEXT_ID: u32 = 0x4E43_4C00;
pub const GAME_LOG_TEXT_GROUP: u32 = 4;
pub const GAME_LOG_TEXT_MAX_UNITS: usize = 120;
pub const UI_TEXT_LOOKUP_FUNCTION_START: [u8; 23] = [
    0x4C, 0x8B, 0xC9, 0x41, 0x83, 0xF8, 0x25, 0x77, 0x13, 0x85, 0xD2, 0x74, 0x0F, 0x49, 0x63, 0xC0,
    0x48, 0x6B, 0xC8, 0x58, 0x49, 0x03, 0xC9,
];
pub const HUD_LOG_POST_TEXT_ID_FUNCTION_START: [u8; 28] = [
    0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x10, 0x01, 0x00, 0x00, 0x48, 0xC7, 0x44, 0x24, 0x20,
    0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18,
];
pub const OVERLAY_LOG_VISIBLE_MS: u64 = 5000;
pub const OVERLAY_LOG_FADE_OUT_MS: u64 = 1200;
pub const OVERLAY_LOG_FADE_IN_MS: u64 = 150;
pub const OVERLAY_LOG_MAX_LINES: usize = 6;
pub const OVERLAY_LOG_DUPLICATE_WINDOW_MS: u64 = 1500;
pub const OVERLAY_LOG_MAX_TEXT_CHARS: usize = 72;
pub const OVERLAY_LOG_LEFT_MARGIN_REFERENCE_PIXELS: f32 = 48.0;
pub const OVERLAY_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS: f32 = 300.0;
pub const OVERLAY_LOG_TEXT_CELL_REFERENCE_PIXELS: f32 = 46.0;
pub const OVERLAY_LOG_LINE_HEIGHT_REFERENCE_PIXELS: f32 = 34.0;
pub const OVERLAY_LOG_PADDING_REFERENCE_PIXELS: f32 = 10.0;

mod layout;
mod log;

pub use layout::{EventLogLayout, event_log_layout};
pub use log::{EventLog, VisibleEvent};

pub const EVENT_LOG_VISIBLE_MS: u64 = 5000;
pub const EVENT_LOG_FADE_OUT_MS: u64 = 1200;
pub const EVENT_LOG_FADE_IN_MS: u64 = 150;
pub const EVENT_LOG_MAX_ENTRIES: usize = 6;
pub const EVENT_LOG_DUPLICATE_WINDOW_MS: u64 = 1500;
pub const EVENT_LOG_MAX_TEXT_CHARS: usize = 72;
pub const EVENT_LOG_LEFT_MARGIN_REFERENCE_PIXELS: f32 = 48.0;
pub const EVENT_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS: f32 = 120.0;
pub const EVENT_LOG_TEXT_SIZE_REFERENCE_PIXELS: f32 = 22.0;
pub const EVENT_LOG_LINE_HEIGHT_REFERENCE_PIXELS: f32 = 30.0;
pub const EVENT_LOG_PADDING_REFERENCE_PIXELS: f32 = 8.0;

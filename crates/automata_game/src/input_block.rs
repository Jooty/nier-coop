mod clear_span;
mod spans_to_clear;

pub use clear_span::ClearSpan;
pub use spans_to_clear::spans_to_clear;

pub const PAD_RECORD_COUNT: u32 = 4;
pub const PAD_RECORD_STRIDE: u32 = 0x38;

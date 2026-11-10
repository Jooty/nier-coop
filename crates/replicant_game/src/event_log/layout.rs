use crate::event_log::{
    EVENT_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS, EVENT_LOG_LEFT_MARGIN_REFERENCE_PIXELS,
    EVENT_LOG_LINE_HEIGHT_REFERENCE_PIXELS, EVENT_LOG_PADDING_REFERENCE_PIXELS,
    EVENT_LOG_TEXT_SIZE_REFERENCE_PIXELS,
};
use crate::party_frame::{FONT_CELL_UNITS, REFERENCE_SCREEN_HEIGHT};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventLogLayout {
    pub left: f32,
    pub bottom: f32,
    pub text_scale: f32,
    pub line_height: f32,
    pub padding: f32,
    pub scale: f32,
}

impl EventLogLayout {
    pub fn line_top(&self, lines_from_bottom: usize) -> f32 {
        self.bottom - self.line_height * (lines_from_bottom as f32 + 1.0)
    }
}

pub fn event_log_layout(screen_height: f32) -> EventLogLayout {
    let scale = screen_height / REFERENCE_SCREEN_HEIGHT;
    EventLogLayout {
        left: EVENT_LOG_LEFT_MARGIN_REFERENCE_PIXELS * scale,
        bottom: screen_height - EVENT_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS * scale,
        text_scale: EVENT_LOG_TEXT_SIZE_REFERENCE_PIXELS * scale / FONT_CELL_UNITS,
        line_height: EVENT_LOG_LINE_HEIGHT_REFERENCE_PIXELS * scale,
        padding: EVENT_LOG_PADDING_REFERENCE_PIXELS * scale,
        scale,
    }
}

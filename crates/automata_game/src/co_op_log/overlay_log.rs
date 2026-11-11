use std::collections::VecDeque;

use crate::co_op_log::{
    OVERLAY_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS, OVERLAY_LOG_DUPLICATE_WINDOW_MS,
    OVERLAY_LOG_FADE_IN_MS, OVERLAY_LOG_FADE_OUT_MS, OVERLAY_LOG_LEFT_MARGIN_REFERENCE_PIXELS,
    OVERLAY_LOG_LINE_HEIGHT_REFERENCE_PIXELS, OVERLAY_LOG_MAX_LINES, OVERLAY_LOG_MAX_TEXT_CHARS,
    OVERLAY_LOG_PADDING_REFERENCE_PIXELS, OVERLAY_LOG_TEXT_CELL_REFERENCE_PIXELS,
    OVERLAY_LOG_VISIBLE_MS,
};
use crate::partner_hud::HUD_REFERENCE_SCREEN_HEIGHT;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    text: String,
    posted_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleLine {
    pub text: String,
    pub alpha: f32,
}

/// The lines the overlay draws itself when the game's own log is not used.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverlayLog {
    entries: VecDeque<Entry>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayLogLayout {
    pub left: f32,
    pub bottom: f32,
    pub text_cell: f32,
    pub line_height: f32,
    pub padding: f32,
    pub scale: f32,
}

impl OverlayLog {
    pub const fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    pub fn post(&mut self, text: &str, now_ms: u64) -> bool {
        let text: String = text
            .chars()
            .filter(|character| !character.is_control())
            .take(OVERLAY_LOG_MAX_TEXT_CHARS)
            .collect();
        if text.is_empty() {
            return false;
        }
        let repeated = self.entries.iter().any(|entry| {
            entry.text == text
                && now_ms.saturating_sub(entry.posted_ms) < OVERLAY_LOG_DUPLICATE_WINDOW_MS
        });
        if repeated {
            return false;
        }
        if self.entries.len() >= OVERLAY_LOG_MAX_LINES {
            self.entries.pop_front();
        }
        self.entries.push_back(Entry {
            text,
            posted_ms: now_ms,
        });
        true
    }

    pub fn visible(&mut self, now_ms: u64) -> Vec<VisibleLine> {
        self.entries
            .retain(|entry| now_ms.saturating_sub(entry.posted_ms) < OVERLAY_LOG_VISIBLE_MS);
        self.entries
            .iter()
            .map(|entry| VisibleLine {
                text: entry.text.clone(),
                alpha: alpha_at(now_ms.saturating_sub(entry.posted_ms)),
            })
            .collect()
    }
}

impl OverlayLogLayout {
    pub fn line_top(&self, lines_from_bottom: usize) -> f32 {
        self.bottom - self.line_height * (lines_from_bottom as f32 + 1.0)
    }
}

pub fn overlay_log_layout(screen_height: f32) -> OverlayLogLayout {
    let scale = screen_height / HUD_REFERENCE_SCREEN_HEIGHT;
    OverlayLogLayout {
        left: OVERLAY_LOG_LEFT_MARGIN_REFERENCE_PIXELS * scale,
        bottom: screen_height - OVERLAY_LOG_BOTTOM_MARGIN_REFERENCE_PIXELS * scale,
        text_cell: OVERLAY_LOG_TEXT_CELL_REFERENCE_PIXELS * scale,
        line_height: OVERLAY_LOG_LINE_HEIGHT_REFERENCE_PIXELS * scale,
        padding: OVERLAY_LOG_PADDING_REFERENCE_PIXELS * scale,
        scale,
    }
}

fn alpha_at(age_ms: u64) -> f32 {
    let fade_out_start = OVERLAY_LOG_VISIBLE_MS - OVERLAY_LOG_FADE_OUT_MS;
    let fade_in = (age_ms as f32 / OVERLAY_LOG_FADE_IN_MS as f32).min(1.0);
    let fade_out = if age_ms <= fade_out_start {
        1.0
    } else {
        (OVERLAY_LOG_VISIBLE_MS.saturating_sub(age_ms)) as f32 / OVERLAY_LOG_FADE_OUT_MS as f32
    };
    (fade_in * fade_out).clamp(0.0, 1.0)
}

use std::sync::atomic::{AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rc_game::marker::{CameraState, FRAME_RING_SLOTS, FrameKey, FrameRing};

use crate::diagnostics::log_line;

pub const OFFSET_AUTO: u8 = u8::MAX;
const MAX_MANUAL_OFFSET: u8 = 3;
const SUMMARY_INTERVAL: Duration = Duration::from_secs(5);

static TICKS: Mutex<FrameRing<PresentedFrame>> = Mutex::new(FrameRing::new());
static PRESENTED: Mutex<Option<PresentedFrame>> = Mutex::new(None);
static OFFSET: AtomicU8 = AtomicU8::new(OFFSET_AUTO);
static LAST_SUMMARY: Mutex<Option<Instant>> = Mutex::new(None);
static SET_VIEW_CALLS: AtomicU64 = AtomicU64::new(0);
static UPLOAD_MATCHED: AtomicU64 = AtomicU64::new(0);
static UPLOAD_MISSED: AtomicU64 = AtomicU64::new(0);
static MATCHED_AGE_SUM: AtomicU64 = AtomicU64::new(0);
static MARKER_MATCHED: AtomicU64 = AtomicU64::new(0);
static MARKER_OFFSET: AtomicU64 = AtomicU64::new(0);
static MARKER_LIVE_READ: AtomicU64 = AtomicU64::new(0);
static SLOTS_SEEN: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy)]
pub struct PresentedFrame {
    pub camera: CameraState,
    pub puppet_position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    Matched,
    Offset(u8),
    LiveRead,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CameraStats {
    pub set_view_calls: u64,
    pub upload_matched: u64,
    pub upload_missed: u64,
    pub matched_age_sum: u64,
    pub marker_matched: u64,
    pub marker_offset: u64,
    pub marker_live_read: u64,
    pub slots_seen: u32,
}

pub(super) fn record_tick(slot: i32, key: FrameKey, frame: PresentedFrame) {
    SET_VIEW_CALLS.fetch_add(1, Ordering::Relaxed);
    if (0..32).contains(&slot) {
        SLOTS_SEEN.fetch_or(1 << slot, Ordering::Relaxed);
    }
    if let Ok(mut ring) = TICKS.try_lock() {
        ring.push(key, frame);
    }
}

pub(super) fn mark_rendered(key: &FrameKey) {
    let found = TICKS
        .try_lock()
        .ok()
        .and_then(|ring| ring.find_with_age(key));
    let Some((frame, age)) = found else {
        UPLOAD_MISSED.fetch_add(1, Ordering::Relaxed);
        return;
    };
    UPLOAD_MATCHED.fetch_add(1, Ordering::Relaxed);
    MATCHED_AGE_SUM.fetch_add(age as u64, Ordering::Relaxed);
    if let Ok(mut presented) = PRESENTED.try_lock() {
        *presented = Some(frame);
    }
}

pub fn camera_offset() -> u8 {
    OFFSET.load(Ordering::Relaxed)
}

pub fn set_camera_offset(offset: u8) {
    let clamped = if offset == OFFSET_AUTO {
        OFFSET_AUTO
    } else {
        offset.min(MAX_MANUAL_OFFSET)
    };
    OFFSET.store(clamped, Ordering::Relaxed);
}

pub fn frame_for_marker() -> Option<(PresentedFrame, FrameSource)> {
    let offset = camera_offset();
    if offset == OFFSET_AUTO {
        let presented = *PRESENTED.lock().unwrap_or_else(PoisonError::into_inner);
        return presented.map(|frame| (frame, FrameSource::Matched));
    }
    let ring = TICKS.lock().unwrap_or_else(PoisonError::into_inner);
    ring.nth_back(usize::from(offset))
        .map(|frame| (frame, FrameSource::Offset(offset)))
}

pub fn note_marker_source(source: FrameSource) {
    let counter = match source {
        FrameSource::Matched => &MARKER_MATCHED,
        FrameSource::Offset(_) => &MARKER_OFFSET,
        FrameSource::LiveRead => &MARKER_LIVE_READ,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub fn camera_stats() -> CameraStats {
    CameraStats {
        set_view_calls: SET_VIEW_CALLS.load(Ordering::Relaxed),
        upload_matched: UPLOAD_MATCHED.load(Ordering::Relaxed),
        upload_missed: UPLOAD_MISSED.load(Ordering::Relaxed),
        matched_age_sum: MATCHED_AGE_SUM.load(Ordering::Relaxed),
        marker_matched: MARKER_MATCHED.load(Ordering::Relaxed),
        marker_offset: MARKER_OFFSET.load(Ordering::Relaxed),
        marker_live_read: MARKER_LIVE_READ.load(Ordering::Relaxed),
        slots_seen: SLOTS_SEEN.load(Ordering::Relaxed),
    }
}

pub fn offset_label(offset: u8) -> String {
    if offset == OFFSET_AUTO {
        "auto".to_owned()
    } else {
        format!("{offset} back")
    }
}

pub fn stats_text(stats: &CameraStats) -> String {
    let average_age = if stats.upload_matched == 0 {
        0.0
    } else {
        stats.matched_age_sum as f64 / stats.upload_matched as f64
    };
    format!(
        "camera frames: {} pushed, uploads matched {} missed {} (avg {average_age:.2} back of {FRAME_RING_SLOTS}); \
         marker used matched {} offset {} live-read {}; slots {:#x}; offset {}",
        stats.set_view_calls,
        stats.upload_matched,
        stats.upload_missed,
        stats.marker_matched,
        stats.marker_offset,
        stats.marker_live_read,
        stats.slots_seen,
        offset_label(camera_offset()),
    )
}

pub fn log_summary_if_due() {
    let mut last = LAST_SUMMARY.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if last.is_some_and(|at| now.saturating_duration_since(at) < SUMMARY_INTERVAL) {
        return;
    }
    *last = Some(now);
    drop(last);
    let stats = camera_stats();
    if stats.set_view_calls > 0 || stats.upload_missed > 0 {
        log_line(&stats_text(&stats));
    }
}

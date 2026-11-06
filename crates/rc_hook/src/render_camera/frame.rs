use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rc_game::marker::{CameraState, FRAME_RING_SLOTS, FrameKey, FrameRing};

use crate::diagnostics::log_line;
use crate::render_camera::hooks::read_drawn_key;

pub const OFFSET_AUTO: u8 = u8::MAX;
const MAX_MANUAL_OFFSET: u8 = 3;
const SUMMARY_INTERVAL: Duration = Duration::from_secs(5);

static TICKS: Mutex<FrameRing<PresentedFrame>> = Mutex::new(FrameRing::new());
static PRESENTED: Mutex<Option<PresentedFrame>> = Mutex::new(None);
static OFFSET: AtomicU8 = AtomicU8::new(OFFSET_AUTO);
static TAG_SYNC_USES_DRAWN_FRAME: AtomicBool = AtomicBool::new(true);
static LAST_SUMMARY: Mutex<Option<Instant>> = Mutex::new(None);
static SET_VIEW_CALLS: AtomicU64 = AtomicU64::new(0);
static UPLOAD_MATCHED: AtomicU64 = AtomicU64::new(0);
static UPLOAD_MISSED: AtomicU64 = AtomicU64::new(0);
static MATCHED_AGE_SUM: AtomicU64 = AtomicU64::new(0);
static MARKER_MATCHED: AtomicU64 = AtomicU64::new(0);
static MARKER_OFFSET: AtomicU64 = AtomicU64::new(0);
static MARKER_LIVE_READ: AtomicU64 = AtomicU64::new(0);
static MARKER_DRAWN: AtomicU64 = AtomicU64::new(0);
static DRAWN_AGE_SUM: AtomicU64 = AtomicU64::new(0);
static DRAWN_NOT_FOUND: AtomicU64 = AtomicU64::new(0);
static POSITION_FROM_FRAME: AtomicU64 = AtomicU64::new(0);
static POSITION_FROM_NETWORK: AtomicU64 = AtomicU64::new(0);
static SLOTS_SEEN: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy)]
pub struct PresentedFrame {
    pub camera: CameraState,
    pub puppet_position: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameSource {
    Drawn(usize),
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
    pub marker_drawn: u64,
    pub drawn_age_sum: u64,
    pub drawn_not_found: u64,
    pub position_from_frame: u64,
    pub position_from_network: u64,
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

pub fn tag_sync_uses_drawn_frame() -> bool {
    TAG_SYNC_USES_DRAWN_FRAME.load(Ordering::Relaxed)
}

pub fn set_tag_sync_uses_drawn_frame(enabled: bool) {
    TAG_SYNC_USES_DRAWN_FRAME.store(enabled, Ordering::Relaxed);
}

pub fn frame_for_marker() -> Option<(PresentedFrame, FrameSource)> {
    let offset = camera_offset();
    if offset == OFFSET_AUTO {
        if tag_sync_uses_drawn_frame() {
            if let Some(found) = drawn_frame() {
                return Some(found);
            }
            DRAWN_NOT_FOUND.fetch_add(1, Ordering::Relaxed);
        }
        let presented = *PRESENTED.lock().unwrap_or_else(PoisonError::into_inner);
        return presented.map(|frame| (frame, FrameSource::Matched));
    }
    let ring = TICKS.lock().unwrap_or_else(PoisonError::into_inner);
    ring.nth_back(usize::from(offset))
        .map(|frame| (frame, FrameSource::Offset(offset)))
}

fn drawn_frame() -> Option<(PresentedFrame, FrameSource)> {
    let key = read_drawn_key()?;
    let ring = TICKS.lock().unwrap_or_else(PoisonError::into_inner);
    ring.find_with_age(&key)
        .map(|(frame, age)| (frame, FrameSource::Drawn(age)))
}

pub fn note_marker_source(source: FrameSource) {
    if let FrameSource::Drawn(age) = source {
        DRAWN_AGE_SUM.fetch_add(age as u64, Ordering::Relaxed);
    }
    let counter = match source {
        FrameSource::Drawn(_) => &MARKER_DRAWN,
        FrameSource::Matched => &MARKER_MATCHED,
        FrameSource::Offset(_) => &MARKER_OFFSET,
        FrameSource::LiveRead => &MARKER_LIVE_READ,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub fn note_position_source(from_frame: bool) {
    let counter = if from_frame {
        &POSITION_FROM_FRAME
    } else {
        &POSITION_FROM_NETWORK
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
        marker_drawn: MARKER_DRAWN.load(Ordering::Relaxed),
        drawn_age_sum: DRAWN_AGE_SUM.load(Ordering::Relaxed),
        drawn_not_found: DRAWN_NOT_FOUND.load(Ordering::Relaxed),
        position_from_frame: POSITION_FROM_FRAME.load(Ordering::Relaxed),
        position_from_network: POSITION_FROM_NETWORK.load(Ordering::Relaxed),
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

fn tag_sync_label(uses_drawn_frame: bool) -> &'static str {
    if uses_drawn_frame { "new" } else { "old" }
}

pub fn stats_text(stats: &CameraStats) -> String {
    let average_age = average(stats.matched_age_sum, stats.upload_matched);
    let drawn_age = average(stats.drawn_age_sum, stats.marker_drawn);
    let counts = format!(
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
    );
    format!(
        "{counts}; tag sync {}: camera from drawn frame {} (avg {drawn_age:.2} back), drawn frame not found {}; \
         tag position from frame {} network {}",
        tag_sync_label(tag_sync_uses_drawn_frame()),
        stats.marker_drawn,
        stats.drawn_not_found,
        stats.position_from_frame,
        stats.position_from_network,
    )
}

fn average(sum: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        sum as f64 / count as f64
    }
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

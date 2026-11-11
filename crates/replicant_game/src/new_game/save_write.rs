use crate::new_game::{SAVE_WRITE_BUSY_GRACE_MS, SAVE_WRITE_TIMEOUT_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWriteProgress {
    Wait,
    Done,
    TimedOut,
}

pub fn save_write_progress(
    saving_seen: bool,
    saving_now: bool,
    since_request_ms: u64,
) -> SaveWriteProgress {
    if saving_now {
        return if since_request_ms >= SAVE_WRITE_TIMEOUT_MS {
            SaveWriteProgress::TimedOut
        } else {
            SaveWriteProgress::Wait
        };
    }
    if saving_seen || since_request_ms >= SAVE_WRITE_BUSY_GRACE_MS {
        SaveWriteProgress::Done
    } else {
        SaveWriteProgress::Wait
    }
}

use crate::save_files::{RELOAD_BUSY_GRACE_MS, RELOAD_TIMEOUT_MS};

const SAVE_MANAGER_IDLE: i32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadProgress {
    Request,
    Wait,
    Loaded,
    Failed(u8),
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReloadInputs {
    pub requested: bool,
    pub busy_seen: bool,
    pub waited_ms: u64,
    pub since_request_ms: u64,
    pub manager_state: i32,
    pub result: u8,
}

pub fn reload_progress(inputs: ReloadInputs) -> ReloadProgress {
    if inputs.waited_ms >= RELOAD_TIMEOUT_MS {
        return ReloadProgress::TimedOut;
    }
    let idle = inputs.manager_state == SAVE_MANAGER_IDLE;
    if !inputs.requested {
        return if idle {
            ReloadProgress::Request
        } else {
            ReloadProgress::Wait
        };
    }
    let read_had_time_to_start =
        inputs.busy_seen || inputs.since_request_ms >= RELOAD_BUSY_GRACE_MS;
    if !idle || !read_had_time_to_start {
        return ReloadProgress::Wait;
    }
    if inputs.result == 0 {
        ReloadProgress::Loaded
    } else {
        ReloadProgress::Failed(inputs.result)
    }
}

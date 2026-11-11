use crate::downed::{
    BLOWN_DOWN_STATE_HANDLER_START, DOWNED_POSE_START_INTERVAL_MS, DOWNED_POSE_STATE,
    STATE_CASE_CALL_START,
};
use crate::partner::{RemoteStateIds, code_starts_with};

/// What the hook reads of the game's handler of the pose state before it trusts the pose.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PoseHandlerReading {
    pub state_case_rva: Option<u32>,
    pub state_case_bytes: Vec<u8>,
    pub handler_bytes: Vec<u8>,
}

/// The blown-down state from its first phase: the game's own fall, landing and lying motion.
pub fn downed_pose_start() -> RemoteStateIds {
    RemoteStateIds {
        state: DOWNED_POSE_STATE as u16,
        variant: 0,
        b: 0,
        c: 0,
    }
}

/// A body kept down leaves the pose state by no start at all: the handler's own get-up start,
/// a hit reaction and a start from the player's buttons all come through the animation start.
pub fn start_is_refused_in_the_downed_pose(kept_down: bool, current_state: i32) -> bool {
    kept_down && current_state == DOWNED_POSE_STATE
}

pub fn pose_is_started_again(current_state: i32, now_ms: u64, last_start_ms: Option<u64>) -> bool {
    current_state != DOWNED_POSE_STATE
        && last_start_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= DOWNED_POSE_START_INTERVAL_MS)
}

/// The function a case of the state dispatcher calls (`mov rcx, rdi; call rel32`), as an RVA.
pub fn handler_called_by_the_state_case(case_rva: u32, case_bytes: &[u8]) -> Option<u32> {
    let call_start = STATE_CASE_CALL_START.len();
    if !code_starts_with(case_bytes, &STATE_CASE_CALL_START) {
        return None;
    }
    let relative = case_bytes.get(call_start..call_start + 4)?;
    let relative = i32::from_le_bytes(relative.try_into().ok()?);
    let after_the_call = case_rva.checked_add(call_start as u32 + 4)?;
    after_the_call.checked_add_signed(relative)
}

pub fn pose_handler_is_the_noted_one(reading: &PoseHandlerReading, noted_handler_rva: u32) -> bool {
    reading
        .state_case_rva
        .and_then(|case| handler_called_by_the_state_case(case, &reading.state_case_bytes))
        == Some(noted_handler_rva)
        && code_starts_with(&reading.handler_bytes, &BLOWN_DOWN_STATE_HANDLER_START)
}

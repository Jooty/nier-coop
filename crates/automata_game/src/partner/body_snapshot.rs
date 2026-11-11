use crate::downed::DownedSignal;
use crate::partner::{HIGHEST_SPEED_SENT, HIGHEST_STATE_ID, PodAim, RecordedStart};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BodySnapshot {
    pub held: u32,
    pub pressed: u32,
    pub speed: f32,
    pub world_facing: f32,
    pub position: [f32; 3],
    pub body_facing: f32,
    pub state: u16,
    /// The hero's own newest animation start, when the sender records them.
    pub last_start: Option<RecordedStart>,
    pub health: i32,
    pub max_health: i32,
    /// No cutscene, event or scene change holds the sender's hero.
    pub free: bool,
    pub phase_id: i32,
    /// The sender's game shows its pause menu or a paused screen.
    pub paused: bool,
    /// A jump of the sender's is pending or under way: its position is not in the place the partner is in.
    pub jump_pending: bool,
    /// The ray the sender's own pod shot aimed along a moment ago; none when its pod aimed at nothing.
    pub pod_aim: Option<PodAim>,
    pub downed: DownedSignal,
}

pub fn state_id_for_wire(state: i32) -> u16 {
    u16::try_from(state)
        .ok()
        .filter(|state| *state <= HIGHEST_STATE_ID)
        .unwrap_or(0)
}

pub fn speed_for_wire(speed: f32) -> f32 {
    if speed.is_finite() {
        speed.clamp(0.0, HIGHEST_SPEED_SENT)
    } else {
        0.0
    }
}

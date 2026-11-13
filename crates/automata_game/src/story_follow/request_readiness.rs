use crate::start::RUNNING_SCENE_STATE;
use crate::story_follow::{NO_REQUESTED_PHASE_ID, PHASE_SYSTEM_IDLE_STATE};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestReadiness {
    pub scene_state: i32,
    pub phase_system_state: i32,
    pub requested_phase_id: i32,
    pub pending_phase_id: i32,
    pub current_phase_id: i32,
    pub pending_name: Option<String>,
    pub current_name: Option<String>,
}

impl RequestReadiness {
    pub fn can_take_a_request(&self) -> bool {
        self.is_idle() && self.pending_equals_current()
    }

    pub fn is_idle_with_pending_unequal_to_current(&self) -> bool {
        self.is_idle() && !self.pending_equals_current()
    }

    fn is_idle(&self) -> bool {
        self.scene_state == RUNNING_SCENE_STATE
            && self.phase_system_state == PHASE_SYSTEM_IDLE_STATE
            && self.requested_phase_id == NO_REQUESTED_PHASE_ID
    }

    fn pending_equals_current(&self) -> bool {
        self.pending_phase_id == self.current_phase_id
            && self.pending_name.is_some()
            && self.pending_name == self.current_name
    }
}

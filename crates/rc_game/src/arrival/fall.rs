use crate::Position;
use crate::arrival::{FALL_DEPTH_BELOW_REFERENCE, FALL_NO_GROUND_MS, FALL_RECOVERY_COOLDOWN_MS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FallInputs {
    pub now_ms: u64,
    pub local_phase: Option<u16>,
    pub local: Position,
    pub ground_under_local: bool,
    pub partner_y_in_same_phase: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FallGuard {
    phase: Option<u16>,
    last_grounded: Option<Position>,
    no_ground_since_ms: Option<u64>,
    quiet_until_ms: u64,
}

impl FallGuard {
    pub const fn new() -> Self {
        Self {
            phase: None,
            last_grounded: None,
            no_ground_since_ms: None,
            quiet_until_ms: 0,
        }
    }

    pub fn last_grounded(&self) -> Option<Position> {
        self.last_grounded
    }

    pub fn has_fallen(&mut self, inputs: &FallInputs) -> bool {
        if inputs.local_phase != self.phase {
            *self = Self {
                phase: inputs.local_phase,
                ..Self::new()
            };
        }
        if inputs.ground_under_local {
            self.last_grounded = Some(inputs.local);
            self.no_ground_since_ms = None;
            return false;
        }
        let since = *self.no_ground_since_ms.get_or_insert(inputs.now_ms);
        let reference_y = inputs
            .partner_y_in_same_phase
            .or(self.last_grounded.map(|spot| spot.y));
        let deep_enough = reference_y
            .is_some_and(|reference| inputs.local.y < reference - FALL_DEPTH_BELOW_REFERENCE);
        let long_enough = inputs.now_ms >= since + FALL_NO_GROUND_MS;
        if deep_enough && long_enough && inputs.now_ms >= self.quiet_until_ms {
            self.quiet_until_ms = inputs.now_ms + FALL_RECOVERY_COOLDOWN_MS;
            self.no_ground_since_ms = None;
            return true;
        }
        false
    }
}

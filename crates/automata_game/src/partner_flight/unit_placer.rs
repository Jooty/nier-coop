use crate::partner::{PositionCorrection, PositionCorrector, facing_needs_write};

/// Corrects the body's flight unit toward the partner's unit by the rule and limits of the body's
/// own position correction; a unit is never teleported by the game, only written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitPlacer {
    corrector: PositionCorrector,
}

impl UnitPlacer {
    /// The position to write into the unit, if any.
    pub fn position_to_write(
        &mut self,
        now_ms: u64,
        local: [f32; 3],
        remote: [f32; 3],
    ) -> Option<[f32; 3]> {
        match self.corrector.correct(now_ms, local, remote) {
            PositionCorrection::Leave => None,
            PositionCorrection::Nudge(position) => Some(position),
            PositionCorrection::Teleport => Some(remote),
        }
    }

    pub fn yaw_to_write(local: f32, remote: f32) -> Option<f32> {
        facing_needs_write(local, remote).then_some(remote)
    }

    pub fn restart(&mut self) {
        self.corrector = PositionCorrector::default();
    }
}

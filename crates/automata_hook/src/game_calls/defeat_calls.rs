use automata_game::address::{
    DEFEAT_FLOW_STATE_VALUE, DEFEAT_FLOW_STEP_VALUE, RESTORE_POINT_POINTER_VALUE,
};
use automata_game::defeat::{
    BODY_MAX_HEALTH_BASE_OFFSET, BODY_MAX_HEALTH_BONUS_OFFSET, DefeatFlow, RESTORE_POINT_MARK,
    RestorePoint,
};

use crate::game_calls::GameCalls;

impl GameCalls {
    pub fn read_defeat_flow(&self) -> Option<DefeatFlow> {
        Some(DefeatFlow {
            state: self.read_i32(DEFEAT_FLOW_STATE_VALUE).ok()?,
            step: self.read_i32(DEFEAT_FLOW_STEP_VALUE).ok()?,
        })
    }

    /// What the game's continue after a defeat tests first: the mark at the start of its buffer.
    pub fn read_restore_point(&self) -> RestorePoint {
        let Ok(buffer) = self.read_u64(RESTORE_POINT_POINTER_VALUE) else {
            return RestorePoint::Unreadable;
        };
        if buffer == 0 {
            return RestorePoint::Absent;
        }
        match self.read_bytes_at(buffer, 2).as_deref() {
            Ok(&[low, high]) if u16::from_le_bytes([low, high]) == RESTORE_POINT_MARK => {
                RestorePoint::Present
            }
            Ok(_) => RestorePoint::Absent,
            Err(_) => RestorePoint::Unreadable,
        }
    }

    /// The sum the body's own "most health" getter returns.
    pub fn read_max_health(&self, body: usize) -> Option<i32> {
        let int = |offset| self.read_i32_at(self.body_field(body, offset)).ok();
        Some(int(BODY_MAX_HEALTH_BASE_OFFSET)?.wrapping_add(int(BODY_MAX_HEALTH_BONUS_OFFSET)?))
    }
}

use crate::enemies::EnemyId;
use crate::enemy_lead::{ENEMY_PLACE_IS_LOGGED_EVERY_MS, PLACE_LINES_LOGGED_AT_MOST};

/// When the next round of place lines is due, and how many lines are left this session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceLog {
    logged_at_ms: Option<u64>,
    lines_left: u32,
}

impl Default for PlaceLog {
    fn default() -> Self {
        Self {
            logged_at_ms: None,
            lines_left: PLACE_LINES_LOGGED_AT_MOST,
        }
    }
}

impl PlaceLog {
    /// True once every two seconds while lines are left; the caller then logs one round.
    pub fn round_is_due(&mut self, now_ms: u64) -> bool {
        if self.lines_left == 0
            || self
                .logged_at_ms
                .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < ENEMY_PLACE_IS_LOGGED_EVERY_MS)
        {
            return false;
        }
        self.logged_at_ms = Some(now_ms);
        true
    }

    pub fn take_line(&mut self) -> bool {
        if self.lines_left == 0 {
            return false;
        }
        self.lines_left -= 1;
        true
    }
}

/// One place line, the same shape on both copies so a script can pair them by the enemy's id.
pub fn place_line(id: EnemyId, position: [f32; 3], state: [i32; 4], how: &str) -> String {
    format!(
        "enemies: place of {} at {:.2} {:.2} {:.2}, state {:#X} {} {} {}, {how}",
        id.label(),
        position[0],
        position[1],
        position[2],
        state[0],
        state[1],
        state[2],
        state[3]
    )
}

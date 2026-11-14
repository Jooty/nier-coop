use crate::enemies::EnemyId;
use crate::enemy_lead::{ENEMY_PLACE_IS_LOGGED_EVERY_MS, PLACE_LINES_LOGGED_PER_MINUTE};
use crate::minute_line_budget::{LineAdmission, MinuteLineBudget};

const PLACE_LINES_TOPIC: &str = "enemies: place lines";

/// When the next round of place lines is due, and how many lines the minute still allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceLog {
    logged_round: Option<u64>,
    budget: MinuteLineBudget,
}

impl Default for PlaceLog {
    fn default() -> Self {
        Self {
            logged_round: None,
            budget: MinuteLineBudget::new(PLACE_LINES_LOGGED_PER_MINUTE),
        }
    }
}

impl PlaceLog {
    /// True once per two-second step of the wall clock, so two copies on
    /// one clock log their rounds at about the same moment; the caller then logs one round.
    pub fn round_is_due(&mut self, wall_clock_ms: u64) -> bool {
        let round = wall_clock_ms / ENEMY_PLACE_IS_LOGGED_EVERY_MS;
        if self.logged_round == Some(round) {
            return false;
        }
        self.logged_round = Some(round);
        true
    }

    pub fn take_line(&mut self, now_ms: u64) -> LineAdmission {
        self.budget.admit(now_ms, PLACE_LINES_TOPIC)
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

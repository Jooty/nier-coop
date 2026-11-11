use crate::stay_together::distance_between;
use crate::story_calls::SAME_PLACE_RANGE_UNITS;
use crate::story_relay::{PULL_ARRIVAL_SETTLE_LIMIT_MS, PULL_ARRIVAL_SETTLE_MIN_MS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PullArrivalInputs {
    pub now_ms: u64,
    pub position: Option<[f32; 3]>,
    pub in_side_view: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PullArrivalStep {
    Idle,
    Settling,
    Ready { waited_ms: u64 },
    GoBack { to: [f32; 3], waited_ms: u64 },
    DidNotArrive { waited_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Pull {
    came_from: [f32; 3],
    starter_at: [f32; 3],
    starter_in_side_view: bool,
    began_ms: u64,
}

/// The pulled player's side of a pull: it counts as arrived once the game has settled beside
/// the starter in the starter's camera view, and goes back when the game does not give that view.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PullArrival {
    pull: Option<Pull>,
}

impl PullArrival {
    pub const fn new() -> Self {
        Self { pull: None }
    }

    pub fn reset(&mut self) {
        self.pull = None;
    }

    pub fn is_settling(&self) -> bool {
        self.pull.is_some()
    }

    pub fn begin(
        &mut self,
        came_from: [f32; 3],
        starter_at: [f32; 3],
        starter_in_side_view: bool,
        now_ms: u64,
    ) -> bool {
        if self.pull.is_some() {
            return false;
        }
        self.pull = Some(Pull {
            came_from,
            starter_at,
            starter_in_side_view,
            began_ms: now_ms,
        });
        true
    }

    pub fn step(&mut self, inputs: &PullArrivalInputs) -> PullArrivalStep {
        let Some(pull) = self.pull else {
            return PullArrivalStep::Idle;
        };
        let waited_ms = inputs.now_ms.saturating_sub(pull.began_ms);
        let view_matches = inputs.in_side_view == pull.starter_in_side_view;
        if waited_ms < PULL_ARRIVAL_SETTLE_MIN_MS {
            return PullArrivalStep::Settling;
        }
        let near_the_starter = inputs
            .position
            .and_then(|here| distance_between(here, pull.starter_at))
            .is_some_and(|units| units <= SAME_PLACE_RANGE_UNITS);
        if near_the_starter && view_matches {
            self.pull = None;
            return PullArrivalStep::Ready { waited_ms };
        }
        if waited_ms < PULL_ARRIVAL_SETTLE_LIMIT_MS {
            return PullArrivalStep::Settling;
        }
        self.pull = None;
        if near_the_starter {
            PullArrivalStep::GoBack {
                to: pull.came_from,
                waited_ms,
            }
        } else {
            PullArrivalStep::DidNotArrive { waited_ms }
        }
    }
}

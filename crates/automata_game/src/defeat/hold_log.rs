use crate::defeat::{HOLD_LINES_LOGGED_PER_MINUTE, is_hero_defeat_state};
use crate::minute_line_budget::MinuteLineBudget;

const HOLD_LINES_TOPIC: &str = "hold";
use crate::partner::NotFree;

/// What the other player's last state says about why its game holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OtherPlayerHold {
    /// False while its game sends states without a body: a load, a scene change or the title.
    pub has_a_hero: bool,
    pub free: bool,
    pub health: i32,
    pub state: u16,
}

/// A hold lasts while not both players count as free; the partner body is the game's then.
#[derive(Debug, Clone)]
pub struct HoldLog {
    holding: Option<(u64, String)>,
    budget: MinuteLineBudget,
}

impl Default for HoldLog {
    fn default() -> Self {
        Self {
            holding: None,
            budget: MinuteLineBudget::new(HOLD_LINES_LOGGED_PER_MINUTE),
        }
    }
}

impl HoldLog {
    pub fn observe(&mut self, now_ms: u64, both_free: bool, cause: String) -> Vec<String> {
        let line = match (self.holding.take(), both_free) {
            (None, true) => return Vec::new(),
            (None, false) => {
                let line = format!("hold: starts: {cause}");
                self.holding = Some((now_ms, cause));
                line
            }
            (Some((since_ms, cause_before)), true) => format!(
                "hold: ends after {} ms, both players are free (the last cause: {cause_before})",
                now_ms.saturating_sub(since_ms)
            ),
            (Some((since_ms, cause_before)), false) => {
                let line = (cause_before != cause).then(|| format!("hold: goes on, now: {cause}"));
                self.holding = Some((since_ms, cause));
                let Some(line) = line else {
                    return Vec::new();
                };
                line
            }
        };
        self.budget.lines_for(now_ms, HOLD_LINES_TOPIC, line)
    }
}

pub fn hold_cause(
    local: Option<NotFree>,
    local_defeat_flow_runs: bool,
    other: Option<OtherPlayerHold>,
) -> String {
    let mut causes = Vec::new();
    match local {
        Some(reason) if local_defeat_flow_runs => causes.push(format!(
            "the local player: {reason} (the game's defeat flow runs)"
        )),
        Some(reason) => causes.push(format!("the local player: {reason}")),
        None => {}
    }
    match other {
        None => causes.push("the other player: no state heard from its game".to_owned()),
        Some(other) if !other.has_a_hero => causes.push(
            "the other player: its game has no hero in play (a load, a scene change or the title)"
                .to_owned(),
        ),
        Some(other) if other.free => {}
        Some(other) if other.health < 1 || is_hero_defeat_state(i32::from(other.state)) => {
            causes.push(format!(
                "the other player: its game holds it after a defeat (health {}, state {:#X})",
                other.health, other.state
            ));
        }
        Some(other) => causes.push(format!(
            "the other player: its game holds it (a cutscene, an event, a scene change or a menu of the game; health {}, state {:#X})",
            other.health, other.state
        )),
    }
    if causes.is_empty() {
        "both are free, but not yet for long enough without a break".to_owned()
    } else {
        causes.join("; ")
    }
}

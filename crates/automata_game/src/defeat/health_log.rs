use crate::defeat::{HEALTH_CHANGE_WORTH_A_LINE_PERCENT, HEALTH_LINES_LOGGED_PER_MINUTE};
use crate::minute_line_budget::MinuteLineBudget;

/// Picks the health changes of one body that are worth a line: defeat, coming back, big steps.
#[derive(Debug, Clone)]
pub struct HealthLog {
    who: &'static str,
    logged: Option<(i32, i32)>,
    budget: MinuteLineBudget,
    topic: String,
}

impl HealthLog {
    pub fn of(who: &'static str) -> Self {
        Self {
            who,
            logged: None,
            budget: MinuteLineBudget::new(HEALTH_LINES_LOGGED_PER_MINUTE),
            topic: format!("health: {who}"),
        }
    }

    pub fn observe(&mut self, now_ms: u64, health: i32, max_health: i32) -> Vec<String> {
        let who = self.who;
        let line = match self.logged {
            None => format!("health: {who} has {health} of {max_health}"),
            Some((before, _)) if before >= 1 && health < 1 => {
                format!("health: {who} has no health left: defeated (it had {before})")
            }
            Some((before, _)) if before < 1 && health >= 1 => {
                format!("health: {who} is back with {health} of {max_health}")
            }
            Some((before, _)) if is_worth_a_line(before, health, max_health) => {
                let change = health - before;
                let verb = if change < 0 { "lost" } else { "gained" };
                format!(
                    "health: {who} {verb} {} and has {health} of {max_health}",
                    change.abs()
                )
            }
            Some((before, max_before)) if max_before != max_health && before == health => {
                format!("health: the most {who} can have went from {max_before} to {max_health}")
            }
            Some(_) => return Vec::new(),
        };
        self.logged = Some((health, max_health));
        self.budget.lines_for(now_ms, &self.topic, line)
    }
}

fn is_worth_a_line(before: i32, health: i32, max_health: i32) -> bool {
    let step =
        (i64::from(max_health.max(1)) * i64::from(HEALTH_CHANGE_WORTH_A_LINE_PERCENT) / 100).max(1);
    i64::from(health - before).abs() >= step
}

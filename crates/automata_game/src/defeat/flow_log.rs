use crate::defeat::{
    DEFEAT_FLOW_LINES_LOGGED_AT_MOST, DEFEAT_FLOW_STALL_LINE_INTERVAL_MS, DefeatFlow,
    is_hero_defeat_state,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePoint {
    Present,
    Absent,
    Unreadable,
}

/// Turns the readings of each tick into the lines that tell a defeat from its start to its end.
#[derive(Debug, Clone, Default)]
pub struct DefeatFlowLog {
    flow: Option<DefeatFlow>,
    flow_since_ms: u64,
    active_since_ms: u64,
    stall_line_at_ms: u64,
    hero_defeat_state: Option<i32>,
    defeats: u32,
    lines: u32,
}

impl RestorePoint {
    fn label(self) -> &'static str {
        match self {
            Self::Present => {
                "the game holds a restore point, so its flow continues from that point"
            }
            Self::Absent => "the game holds no restore point, so its flow asks for the title",
            Self::Unreadable => "the restore point cannot be read",
        }
    }
}

impl DefeatFlowLog {
    pub fn defeats(&self) -> u32 {
        self.defeats
    }

    pub fn observe(
        &mut self,
        now_ms: u64,
        flow: DefeatFlow,
        hero_state: Option<i32>,
        restore_point: RestorePoint,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        self.observe_hero_state(hero_state, &mut lines);
        self.observe_flow(now_ms, flow, restore_point, &mut lines);
        let room = DEFEAT_FLOW_LINES_LOGGED_AT_MOST.saturating_sub(self.lines) as usize;
        lines.truncate(room);
        self.lines += lines.len() as u32;
        lines
    }

    fn observe_hero_state(&mut self, hero_state: Option<i32>, lines: &mut Vec<String>) {
        let defeat_state = hero_state.filter(|state| is_hero_defeat_state(*state));
        if defeat_state == self.hero_defeat_state {
            return;
        }
        lines.push(match (self.hero_defeat_state, defeat_state) {
            (_, Some(state)) => format!("defeat: the hero is in its defeat state {state:#X}"),
            (Some(before), None) => match hero_state {
                Some(state) => format!(
                    "defeat: the hero left its defeat state {before:#X} and is in state {state:#X}"
                ),
                None => format!(
                    "defeat: the hero that was in its defeat state {before:#X} is gone (a scene change removes it)"
                ),
            },
            (None, None) => return,
        });
        self.hero_defeat_state = defeat_state;
    }

    fn observe_flow(
        &mut self,
        now_ms: u64,
        flow: DefeatFlow,
        restore_point: RestorePoint,
        lines: &mut Vec<String>,
    ) {
        let before = self.flow.replace(flow);
        if before == Some(flow) {
            let stalled_ms = now_ms.saturating_sub(self.flow_since_ms);
            if flow.is_active()
                && now_ms.saturating_sub(self.stall_line_at_ms)
                    >= DEFEAT_FLOW_STALL_LINE_INTERVAL_MS
            {
                self.stall_line_at_ms = now_ms;
                lines.push(format!(
                    "defeat: the game's defeat flow has not moved for {} s, it is at {}",
                    stalled_ms / 1000,
                    flow.describe()
                ));
            }
            return;
        }
        self.flow_since_ms = now_ms;
        self.stall_line_at_ms = now_ms;
        let was_active = before.is_some_and(DefeatFlow::is_active);
        match (was_active, flow.is_active()) {
            (false, true) => {
                self.defeats += 1;
                self.active_since_ms = now_ms;
                lines.push(format!(
                    "defeat: the game's defeat flow starts (defeat {} of this session; {}): {}",
                    self.defeats,
                    restore_point.label(),
                    flow.describe()
                ));
            }
            (true, true) => lines.push(format!("defeat: flow: {}", flow.describe())),
            (true, false) => lines.push(format!(
                "defeat: the game's defeat flow is over after {} ms",
                now_ms.saturating_sub(self.active_since_ms)
            )),
            (false, false) => {}
        }
    }
}

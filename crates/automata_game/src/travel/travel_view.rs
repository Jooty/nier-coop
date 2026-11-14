use crate::travel::{JumpRefusal, JumpRole, TravelPoint, WorldReadings, world_refusal};

const NOTHING_ASKED_STATUS_LINE: &str = "teleport: nothing asked yet";

/// What the game thread last saw, for the menu on the render thread.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TravelView {
    pub role: JumpRole,
    pub world: Option<WorldReadings>,
    pub jump_under_way: bool,
    pub current: Option<TravelPoint>,
    pub partner_body_present: bool,
    /// None until the other player's state has told its phase.
    pub same_phase_as_partner: Option<bool>,
    pub travel_line: String,
    pub move_line: String,
}

impl TravelView {
    pub fn points(&self) -> Vec<TravelPoint> {
        self.world
            .map(|world| world.loaded.points())
            .unwrap_or_default()
    }

    pub fn jump_blocker(&self) -> Option<JumpRefusal> {
        if self.role == JumpRole::Guest {
            return Some(JumpRefusal::NotHost);
        }
        world_refusal(self.jump_under_way, self.world.as_ref())
    }

    pub fn point_blocker(&self, point: TravelPoint) -> Option<JumpRefusal> {
        self.jump_blocker().or_else(|| {
            let loaded = self.world.is_some_and(|world| world.loaded.contains(point));
            (!loaded).then_some(JumpRefusal::PointNotLoaded(point))
        })
    }

    pub fn notes(&self) -> Vec<String> {
        [&self.move_line, &self.travel_line]
            .into_iter()
            .filter(|line| !line.is_empty())
            .cloned()
            .collect()
    }

    pub fn status_line(&self) -> String {
        let notes = self.notes();
        if notes.is_empty() {
            NOTHING_ASKED_STATUS_LINE.to_owned()
        } else {
            format!("teleport: {}", notes.join("; "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestGates {
    pub player_held_by: Option<String>,
    pub save_busy: Option<bool>,
    pub save_failed: Option<bool>,
}

impl GuestGates {
    pub fn why_not_ready(&self) -> Option<String> {
        if let Some(held_by) = &self.player_held_by {
            return Some(format!("the player is not free ({held_by})"));
        }
        match self.save_busy {
            Some(false) => None,
            Some(true) => Some("the save manager is busy".to_owned()),
            None => Some("the save manager could not be read".to_owned()),
        }
    }
}

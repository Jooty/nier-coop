use crate::save_together::GuestGates;
use crate::travel::JumpRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveReadings {
    pub now_ms: u64,
    pub role: JumpRole,
    pub campaign_partner_present: bool,
    pub local_in_play: bool,
    pub partner_in_play: bool,
    pub saves_asked_by_the_game: u32,
    pub host_saves_heard: Vec<u32>,
    pub gates: Option<GuestGates>,
}

impl SaveReadings {
    pub fn both_in_play(&self) -> bool {
        self.campaign_partner_present && self.local_in_play && self.partner_in_play
    }

    pub fn why_the_host_cannot_tell(&self) -> &'static str {
        if !self.campaign_partner_present {
            "no campaign partner is linked"
        } else if !self.local_in_play {
            "this game is not in play"
        } else {
            "the partner is not in play"
        }
    }
}

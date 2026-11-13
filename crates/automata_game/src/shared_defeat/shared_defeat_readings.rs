use crate::defeat::{DefeatFlow, RestorePoint};
use crate::shared_defeat::{DefeatNote, GameOverMenu, RestorePlace};
use crate::travel::JumpRole;

#[derive(Debug, Clone, PartialEq)]
pub struct SharedDefeatReadings {
    pub now_ms: u64,
    pub role: JumpRole,
    pub campaign_partner_present: bool,
    pub flow: Option<DefeatFlow>,
    pub menu: Option<GameOverMenu>,
    pub in_place_retry: bool,
    pub restore_point: RestorePoint,
    pub restore_place: Option<RestorePlace>,
    pub heard: Vec<DefeatNote>,
    pub local_unattended: bool,
}

impl SharedDefeatReadings {
    pub fn campaign_pair(&self) -> bool {
        self.role != JumpRole::Solo && self.campaign_partner_present
    }
}

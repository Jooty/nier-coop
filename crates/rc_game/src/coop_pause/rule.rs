use crate::coop_pause::MENU_WORLD_FREEZE_BITS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PauseView {
    pub partner_connected: bool,
    pub menu_open: bool,
    pub event_scene: bool,
}

impl PauseView {
    pub fn applies(self) -> bool {
        self.partner_connected && self.menu_open && !self.event_scene
    }
}

pub fn effective_control_mask(mask: u32, view: PauseView) -> u32 {
    if view.applies() {
        mask & !MENU_WORLD_FREEZE_BITS
    } else {
        mask
    }
}

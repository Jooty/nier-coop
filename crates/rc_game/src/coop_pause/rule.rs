use crate::coop_pause::{MENU_PLAYER_INPUT_BIT, MENU_WORLD_FREEZE_BITS};

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

/// The control mask the game sees, from its four slots. While no-pause applies, the world-freeze
/// bits of slot 0 (where the menus write) are dropped and the player-input bit takes their place;
/// talk (slot 1), save and load (slot 2) and the system message (slot 3) are left as they are.
pub fn effective_control_mask(slots: [u32; 4], view: PauseView) -> u32 {
    let [menu_slot, talk_slot, save_slot, message_slot] = slots;
    let others = talk_slot | save_slot | message_slot;
    if view.applies() && menu_slot & MENU_WORLD_FREEZE_BITS != 0 {
        (menu_slot & !MENU_WORLD_FREEZE_BITS) | MENU_PLAYER_INPUT_BIT | others
    } else {
        menu_slot | others
    }
}

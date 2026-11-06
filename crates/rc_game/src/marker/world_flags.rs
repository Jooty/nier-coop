use crate::marker::{
    CONTROL_MASK_FIRST_RVA, CONTROL_MASK_SLOT_COUNT, EVENT_SCENE_BYTE_RVA, WORLD_FROZEN_MASK_BIT,
};
use rc_proto::PartnerActivity;

use crate::dialogue::TALK_WINDOW_RVA;
use crate::memory::{GameMemory, MemoryError};
use crate::ui_scenes::read_open_ui_scenes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldFlags {
    pub event_scene: bool,
    pub control_mask: u32,
    pub main_menu_open: bool,
    pub talk_window_open: bool,
}

impl WorldFlags {
    pub fn world_frozen(self) -> bool {
        self.control_mask & WORLD_FROZEN_MASK_BIT != 0
    }

    pub fn activity(self) -> PartnerActivity {
        if self.event_scene {
            PartnerActivity::InCutscene
        } else if self.main_menu_open {
            PartnerActivity::InMenu
        } else if self.talk_window_open {
            PartnerActivity::InDialogue
        } else {
            PartnerActivity::Playing
        }
    }
}

pub fn read_world_flags<M: GameMemory>(memory: &M) -> Result<WorldFlags, MemoryError> {
    let base = memory.module_base();
    let event_scene = memory.read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))? != 0;
    let mut control_mask = 0_u32;
    for slot in 0..CONTROL_MASK_SLOT_COUNT {
        let address = base + u64::from(CONTROL_MASK_FIRST_RVA) + u64::from(slot) * 4;
        control_mask |= u32::from_le_bytes(memory.read_array(address)?);
    }
    let talk_window_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let main_menu_open = read_open_ui_scenes(memory).is_ok_and(|scenes| scenes.pause_menu_open());
    Ok(WorldFlags {
        event_scene,
        control_mask,
        main_menu_open,
        talk_window_open,
    })
}

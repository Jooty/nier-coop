use crate::memory::{GameMemory, MemoryError};
use crate::ui_scenes::{
    PAUSE_MENU_SCENE_IDS, SHORT_SYSTEM_MENU_OPEN_FLAG_OFFSET, SHORT_SYSTEM_MENU_SCENE_ID,
    UI_LINK_NEXT_OFFSET, UI_MANAGER_FIRST_SCENE_LINK_OFFSET, UI_MANAGER_RVA,
    UI_MANAGER_SCENE_LIST_OFFSET, UI_SCENE_FIRST_SEQUENCE_LINK_OFFSET, UI_SCENE_ID_OFFSET,
    UI_SCENE_SEQUENCE_LIST_OFFSET, UI_SCENE_WALK_LIMIT,
};

/// The UI scenes in the UI manager's list, as a bit per scene id, plus the open flag of the
/// short system menu, which stays in the list while closed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OpenUiScenes {
    pub scene_bits: u32,
    pub short_system_menu_open: bool,
}

impl OpenUiScenes {
    pub fn contains(self, scene_id: u8) -> bool {
        u32::from(scene_id) < u32::BITS && self.scene_bits & (1 << scene_id) != 0
    }

    pub fn pause_menu_open(self) -> bool {
        self.short_system_menu_open || PAUSE_MENU_SCENE_IDS.iter().any(|id| self.contains(*id))
    }

    pub fn scene_ids(self) -> Vec<u8> {
        (0..u32::BITS as u8)
            .filter(|id| self.contains(*id))
            .collect()
    }

    fn insert(&mut self, scene_id: u8) {
        if u32::from(scene_id) < u32::BITS {
            self.scene_bits |= 1 << scene_id;
        }
    }
}

/// Walks the UI manager's scene list (a ring of `{owner, next, prev}` links). A manager the game
/// has not built yet holds a null link and reads as no scene open.
pub fn read_open_ui_scenes<M: GameMemory>(memory: &M) -> Result<OpenUiScenes, MemoryError> {
    let manager = memory.module_base() + u64::from(UI_MANAGER_RVA);
    let ring_end = manager + UI_MANAGER_SCENE_LIST_OFFSET;
    let mut scenes = OpenUiScenes::default();
    let mut link = memory.read_u64(manager + UI_MANAGER_FIRST_SCENE_LINK_OFFSET)?;
    for _ in 0..UI_SCENE_WALK_LIMIT {
        if link == 0 || link == ring_end {
            break;
        }
        let scene = memory.read_u64(link)?;
        let scene_id = memory.read_u8(scene + UI_SCENE_ID_OFFSET)?;
        scenes.insert(scene_id);
        if scene_id == SHORT_SYSTEM_MENU_SCENE_ID && short_system_menu_open(memory, scene)? {
            scenes.short_system_menu_open = true;
        }
        link = memory.read_u64(link + UI_LINK_NEXT_OFFSET)?;
    }
    Ok(scenes)
}

fn short_system_menu_open<M: GameMemory>(memory: &M, scene: u64) -> Result<bool, MemoryError> {
    let first = memory.read_u64(scene + UI_SCENE_FIRST_SEQUENCE_LINK_OFFSET)?;
    if first == 0 || first == scene + UI_SCENE_SEQUENCE_LIST_OFFSET {
        return Ok(false);
    }
    let sequence = memory.read_u64(first)?;
    Ok(memory.read_u8(sequence + SHORT_SYSTEM_MENU_OPEN_FLAG_OFFSET)? != 0)
}

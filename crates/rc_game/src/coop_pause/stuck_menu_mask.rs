use crate::coop_pause::{LEFTOVER_PAUSE_MENU_MASK, LEFTOVER_PAUSE_MENU_MASK_GRACE_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskView {
    pub slots: [u32; 4],
    pub extra_ui_scene_open: bool,
    pub ui_dialog_state: i32,
    pub event_scene: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StuckMenuMask {
    leftover_since_ms: Option<u64>,
}

impl StuckMenuMask {
    pub const fn new() -> Self {
        Self {
            leftover_since_ms: None,
        }
    }

    pub fn observe(&mut self, view: MaskView, now_ms: u64) -> Option<u64> {
        if !view.is_leftover_pause_menu_mask() {
            self.leftover_since_ms = None;
            return None;
        }
        let since = *self.leftover_since_ms.get_or_insert(now_ms);
        let stuck_ms = now_ms.saturating_sub(since);
        if stuck_ms < LEFTOVER_PAUSE_MENU_MASK_GRACE_MS {
            return None;
        }
        self.leftover_since_ms = None;
        Some(stuck_ms)
    }
}

impl MaskView {
    pub fn is_leftover_pause_menu_mask(self) -> bool {
        self.slots == [LEFTOVER_PAUSE_MENU_MASK, 0, 0, 0]
            && !self.extra_ui_scene_open
            && self.ui_dialog_state == 0
            && !self.event_scene
    }
}

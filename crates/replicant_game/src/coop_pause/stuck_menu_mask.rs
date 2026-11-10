use crate::coop_pause::{LEFTOVER_PAUSE_MENU_MASK, LEFTOVER_PAUSE_MENU_MASK_GRACE_MS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskView {
    pub slots: [u32; 4],
    pub extra_ui_scene_open: bool,
    pub ui_dialog_state: i32,
    pub event_scene: bool,
    pub talk_window_open: bool,
    pub in_play: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StuckMenuMask {
    leftover_since_ms: Option<u64>,
    other_slot_since: Option<(usize, u32, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StuckOtherSlot {
    pub slot: usize,
    pub mask: u32,
    pub stuck_ms: u64,
}

impl StuckMenuMask {
    pub const fn new() -> Self {
        Self {
            leftover_since_ms: None,
            other_slot_since: None,
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

    pub fn observe_other_slots(&mut self, view: MaskView, now_ms: u64) -> Option<StuckOtherSlot> {
        let Some((slot, mask)) = view.leftover_other_slot() else {
            self.other_slot_since = None;
            return None;
        };
        let (_, _, since) = *self
            .other_slot_since
            .filter(|(seen_slot, seen_mask, _)| (*seen_slot, *seen_mask) == (slot, mask))
            .get_or_insert((slot, mask, now_ms));
        self.other_slot_since = Some((slot, mask, since));
        let stuck_ms = now_ms.saturating_sub(since);
        if stuck_ms < LEFTOVER_PAUSE_MENU_MASK_GRACE_MS {
            return None;
        }
        self.other_slot_since = None;
        Some(StuckOtherSlot {
            slot,
            mask,
            stuck_ms,
        })
    }
}

impl MaskView {
    fn leftover_other_slot(self) -> Option<(usize, u32)> {
        if self.extra_ui_scene_open
            || self.ui_dialog_state != 0
            || self.event_scene
            || self.talk_window_open
            || !self.in_play
        {
            return None;
        }
        self.slots
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, mask)| **mask != 0)
            .map(|(slot, mask)| (slot, *mask))
    }

    pub fn is_leftover_pause_menu_mask(self) -> bool {
        self.slots == [LEFTOVER_PAUSE_MENU_MASK, 0, 0, 0]
            && !self.extra_ui_scene_open
            && self.ui_dialog_state == 0
            && !self.event_scene
            && self.in_play
    }
}

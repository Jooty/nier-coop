use rc_proto::{
    ContainerFlag, ContainerItem, ContainerOpened, MAX_CONTAINER_FLAGS, MAX_CONTAINER_GOLD,
    MAX_CONTAINER_ITEMS, MAX_ITEM_COUNT, StoryArray,
};

use crate::containers::OPEN_WINDOW_MS;
use crate::pickups::is_key_item;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedOpen {
    pub action_id: u16,
    pub phase: u16,
    pub items: Vec<ContainerItem>,
    pub gold: u32,
    pub flags: Vec<ContainerFlag>,
}

#[derive(Debug, Default)]
pub struct OpenWindow {
    open: Option<CollectedOpen>,
    closes_at_ms: u64,
}

impl OpenWindow {
    pub fn begin(&mut self, action_id: u16, phase: u16, now_ms: u64) -> Option<CollectedOpen> {
        if self
            .open
            .as_ref()
            .is_some_and(|open| open.action_id == action_id)
        {
            return None;
        }
        let finished = self.open.take();
        self.open = Some(CollectedOpen {
            action_id,
            phase,
            items: Vec::new(),
            gold: 0,
            flags: Vec::new(),
        });
        self.closes_at_ms = now_ms + OPEN_WINDOW_MS;
        finished
    }

    pub fn is_collecting(&self, now_ms: u64) -> bool {
        self.open.is_some() && now_ms < self.closes_at_ms
    }

    pub fn note_item(&mut self, item_id: u16, count: u8, now_ms: u64) {
        if !self.is_collecting(now_ms) || is_key_item(item_id) || count == 0 {
            return;
        }
        let Some(open) = self.open.as_mut() else {
            return;
        };
        if let Some(existing) = open.items.iter_mut().find(|item| item.item_id == item_id) {
            existing.count = existing.count.saturating_add(count).min(MAX_ITEM_COUNT);
        } else if open.items.len() < MAX_CONTAINER_ITEMS {
            open.items.push(ContainerItem {
                item_id,
                count: count.min(MAX_ITEM_COUNT),
            });
        }
    }

    pub fn note_gold(&mut self, amount: i32, now_ms: u64) {
        if !self.is_collecting(now_ms) {
            return;
        }
        let (Some(open), Ok(amount)) = (self.open.as_mut(), u32::try_from(amount)) else {
            return;
        };
        open.gold = open.gold.saturating_add(amount).min(MAX_CONTAINER_GOLD);
    }

    pub fn note_flag(&mut self, array: StoryArray, id: u16, now_ms: u64) {
        if !self.is_collecting(now_ms) {
            return;
        }
        let Some(open) = self.open.as_mut() else {
            return;
        };
        let flag = ContainerFlag { array, id };
        if open.flags.len() < MAX_CONTAINER_FLAGS && !open.flags.contains(&flag) {
            open.flags.push(flag);
        }
    }

    pub fn take_finished(&mut self, now_ms: u64) -> Option<CollectedOpen> {
        if self.open.is_some() && now_ms >= self.closes_at_ms {
            return self.open.take();
        }
        None
    }

    pub fn reset(&mut self) {
        self.open = None;
    }
}

impl CollectedOpen {
    pub fn into_message(self) -> Option<ContainerOpened> {
        if self.items.is_empty() && self.gold == 0 {
            return None;
        }
        Some(ContainerOpened {
            phase: self.phase,
            action_id: self.action_id,
            items: self.items,
            gold: self.gold,
            flags: self.flags,
        })
    }
}

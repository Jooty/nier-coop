use rc_proto::{ItemCountEntry, ItemCounts, MAX_ITEM_COUNT_ENTRIES, MAX_ITEM_ID};

use crate::pickups::is_key_item;

pub const POLL_HOLD_AFTER_TAKE_MS: u64 = 2_000;
pub const INJECTED_END_TAKE_BLOCK_MS: u64 = 3_000;

const ITEM_ID_SLOT_COUNT: usize = MAX_ITEM_ID as usize + 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemCountTable {
    counts: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnInSplit {
    pub local: u8,
    pub partner: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PollHold {
    item_id: u16,
    frozen_count: u8,
    until_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InjectedEndWindow {
    until_ms: u64,
}

impl Default for ItemCountTable {
    fn default() -> Self {
        Self {
            counts: vec![0; ITEM_ID_SLOT_COUNT],
        }
    }
}

impl ItemCountTable {
    pub fn count(&self, item_id: u16) -> u8 {
        if !is_shared_item(item_id) {
            return 0;
        }
        self.counts[usize::from(item_id)]
    }

    pub fn set(&mut self, item_id: u16, count: u8) -> bool {
        if !is_shared_item(item_id) || self.counts[usize::from(item_id)] == count {
            return false;
        }
        self.counts[usize::from(item_id)] = count;
        true
    }

    pub fn lower(&mut self, item_id: u16, by: u8) {
        let now = self.count(item_id).saturating_sub(by);
        self.set(item_id, now);
    }

    pub fn apply(&mut self, message: &ItemCounts) {
        if message.replaces_all {
            self.clear();
        }
        for entry in &message.entries {
            self.set(entry.item_id, entry.count);
        }
    }

    pub fn clear(&mut self) {
        self.counts.fill(0);
    }

    pub fn full_sync(&self) -> Vec<ItemCounts> {
        let held: Vec<ItemCountEntry> = (0..=MAX_ITEM_ID)
            .filter(|item_id| self.count(*item_id) > 0)
            .map(|item_id| ItemCountEntry {
                item_id,
                count: self.count(item_id),
            })
            .collect();
        if held.is_empty() {
            return vec![ItemCounts {
                replaces_all: true,
                entries: Vec::new(),
            }];
        }
        held.chunks(MAX_ITEM_COUNT_ENTRIES)
            .enumerate()
            .map(|(index, chunk)| ItemCounts {
                replaces_all: index == 0,
                entries: chunk.to_vec(),
            })
            .collect()
    }
}

impl PollHold {
    pub const fn new() -> Self {
        Self {
            item_id: 0,
            frozen_count: 0,
            until_ms: 0,
        }
    }

    pub fn note_take(&mut self, item_id: u16, count_before: u8, now_ms: u64) {
        self.item_id = item_id;
        self.frozen_count = count_before;
        self.until_ms = now_ms + POLL_HOLD_AFTER_TAKE_MS;
    }

    pub fn frozen_count(&self, item_id: u16, now_ms: u64) -> Option<u8> {
        (now_ms < self.until_ms && item_id == self.item_id).then_some(self.frozen_count)
    }
}

impl InjectedEndWindow {
    pub const fn new() -> Self {
        Self { until_ms: 0 }
    }

    pub fn open(&mut self, now_ms: u64) {
        self.until_ms = now_ms + INJECTED_END_TAKE_BLOCK_MS;
    }

    pub fn is_open(&self, now_ms: u64) -> bool {
        now_ms < self.until_ms
    }
}

pub fn is_shared_item(item_id: u16) -> bool {
    item_id <= MAX_ITEM_ID && !is_key_item(item_id)
}

pub fn quest_check_count(local: u8, partner: u8) -> u8 {
    local.saturating_add(partner)
}

pub fn split_turn_in(wanted: u8, local: u8, partner: u8) -> TurnInSplit {
    let from_local = wanted.min(local);
    TurnInSplit {
        local: from_local,
        partner: (wanted - from_local).min(partner),
    }
}

pub fn partner_take_amount(asked: u8, own: u8) -> u8 {
    asked.min(own)
}

pub fn item_poll_runs_here(is_host: bool, partner_shares_phase: bool) -> bool {
    is_host || !partner_shares_phase
}

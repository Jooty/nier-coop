use replicant_proto::{ItemCountEntry, ItemCounts, MAX_ITEM_COUNT_ENTRIES, MAX_ITEM_ID};

use crate::offsets::MENU_QUEST_PAGE_GET_COUNT_CALLER_RVA;
use crate::pickups::is_key_item;

pub const POLL_HOLD_AFTER_TAKE_MS: u64 = 2_000;
pub const PARTNER_END_WAIT_CAP_MS: u64 = 60_000;
pub const INJECTED_END_TAKE_BLOCK_MS: u64 = 3_000;

const ITEM_ID_SLOT_COUNT: usize = MAX_ITEM_ID as usize + 1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemCountTable {
    counts: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartnerShare {
    table: ItemCountTable,
    frozen_for: Option<FrozenFor>,
    pending: Vec<(u16, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FrozenFor {
    end_name: String,
    since_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnInSplit {
    pub local: u8,
    pub partner: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PollHold {
    item_id: u16,
    frozen_count: u8,
    until_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedPartnerTake {
    pub item_id: u16,
    pub amount: u8,
    pub end_name: String,
    pub since_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuedTakeFlushReason {
    EndFinishedWithoutTake,
    Cap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlushedPartnerTake {
    pub take: QueuedPartnerTake,
    pub reason: QueuedTakeFlushReason,
    pub waited_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartnerTakeQueue {
    waiting: Vec<QueuedPartnerTake>,
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

impl PartnerShare {
    pub fn count(&self, item_id: u16) -> u8 {
        self.table.count(item_id)
    }

    pub fn frozen_for(&self) -> Option<&str> {
        self.frozen_for
            .as_ref()
            .map(|frozen| frozen.end_name.as_str())
    }

    pub fn freeze(&mut self, end_name: &str, now_ms: u64) {
        if self.frozen_for().is_some_and(|name| name == end_name) {
            return;
        }
        let since_ms = self
            .frozen_for
            .as_ref()
            .map_or(now_ms, |frozen| frozen.since_ms);
        self.frozen_for = Some(FrozenFor {
            end_name: end_name.to_owned(),
            since_ms,
        });
    }

    pub fn cap_reached(&self, now_ms: u64) -> bool {
        self.frozen_for
            .as_ref()
            .is_some_and(|frozen| now_ms.saturating_sub(frozen.since_ms) >= PARTNER_END_WAIT_CAP_MS)
    }

    pub fn release(&mut self) -> Vec<(u16, u8)> {
        self.frozen_for = None;
        let applied = std::mem::take(&mut self.pending);
        for (item_id, count) in &applied {
            self.table.set(*item_id, *count);
        }
        applied
    }

    pub fn apply(&mut self, message: &ItemCounts) {
        if self.frozen_for.is_none() {
            self.table.apply(message);
            return;
        }
        if message.replaces_all {
            for item_id in 0..=MAX_ITEM_ID {
                let listed = message.entries.iter().any(|entry| entry.item_id == item_id);
                if !listed && self.effective(item_id) > 0 {
                    self.set_frozen(item_id, 0);
                }
            }
        }
        for entry in &message.entries {
            self.set_frozen(entry.item_id, entry.count);
        }
    }

    pub fn lower(&mut self, item_id: u16, by: u8) {
        let now = self.effective(item_id).saturating_sub(by);
        if self.frozen_for.is_some() {
            self.set_frozen(item_id, now);
        } else {
            self.table.set(item_id, now);
        }
    }

    pub fn clear(&mut self) {
        self.table.clear();
        self.pending.clear();
        self.frozen_for = None;
    }

    fn effective(&self, item_id: u16) -> u8 {
        self.pending
            .iter()
            .find(|(pending_id, _)| *pending_id == item_id)
            .map_or_else(|| self.table.count(item_id), |(_, count)| *count)
    }

    fn set_frozen(&mut self, item_id: u16, count: u8) {
        if !is_shared_item(item_id) {
            return;
        }
        self.pending
            .retain(|(pending_id, _)| *pending_id != item_id);
        if count < self.table.count(item_id) {
            self.pending.push((item_id, count));
        } else {
            self.table.set(item_id, count);
        }
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

impl PartnerTakeQueue {
    pub const fn new() -> Self {
        Self {
            waiting: Vec::new(),
        }
    }

    pub fn queue(&mut self, item_id: u16, amount: u8, end_name: &str, now_ms: u64) {
        self.waiting.push(QueuedPartnerTake {
            item_id,
            amount,
            end_name: end_name.to_owned(),
            since_ms: now_ms,
        });
    }

    pub fn is_waiting(&self) -> bool {
        !self.waiting.is_empty()
    }

    pub fn amount_waiting_for(&self, item_id: u16, end_name: &str) -> u8 {
        self.waiting
            .iter()
            .filter(|take| take.item_id == item_id && take.end_name == end_name)
            .fold(0, |sum, take| sum.saturating_add(take.amount))
    }

    pub fn take_in_end(&mut self, item_id: u16, end_name: &str) -> Option<u8> {
        let amount = self.amount_waiting_for(item_id, end_name);
        if amount == 0 {
            return None;
        }
        self.waiting
            .retain(|take| !(take.item_id == item_id && take.end_name == end_name));
        Some(amount)
    }

    pub fn flush_due(
        &mut self,
        now_ms: u64,
        end_finished: impl Fn(&str) -> bool,
    ) -> Vec<FlushedPartnerTake> {
        let mut flushed = Vec::new();
        let mut still_waiting = Vec::new();
        for take in self.waiting.drain(..) {
            let waited_ms = now_ms.saturating_sub(take.since_ms);
            let reason = if waited_ms >= PARTNER_END_WAIT_CAP_MS {
                Some(QueuedTakeFlushReason::Cap)
            } else if end_finished(&take.end_name) {
                Some(QueuedTakeFlushReason::EndFinishedWithoutTake)
            } else {
                None
            };
            match reason {
                Some(reason) => flushed.push(FlushedPartnerTake {
                    take,
                    reason,
                    waited_ms,
                }),
                None => still_waiting.push(take),
            }
        }
        self.waiting = still_waiting;
        flushed
    }

    pub fn clear(&mut self) {
        self.waiting.clear();
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

pub const fn is_menu_quest_page_caller(caller_rva: u32) -> bool {
    caller_rva == MENU_QUEST_PAGE_GET_COUNT_CALLER_RVA
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

use replicant_proto::{ScriptRoll, ScriptRollValue};

use crate::script_rolls::{SHARED_ROLL_EXPIRY_MS, SHARED_ROLL_STORE_MAX_ENTRIES};

#[derive(Clone, Copy, Debug, PartialEq)]
struct StoredRoll {
    roll: ScriptRoll,
    arrived_ms: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SharedRollStore {
    entries: Vec<StoredRoll>,
}

impl SharedRollStore {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn offer(&mut self, roll: ScriptRoll, now_ms: u64) {
        self.drop_expired(now_ms);
        if self.entries.len() >= SHARED_ROLL_STORE_MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(StoredRoll {
            roll,
            arrived_ms: now_ms,
        });
    }

    pub fn take(
        &mut self,
        phase: u16,
        call: u64,
        ordinal: u16,
        now_ms: u64,
    ) -> Option<ScriptRollValue> {
        self.drop_expired(now_ms);
        let position = self.entries.iter().position(|stored| {
            stored.roll.phase == phase && stored.roll.call == call && stored.roll.ordinal == ordinal
        })?;
        Some(self.entries.remove(position).roll.value)
    }

    pub fn forget_other_phases(&mut self, phase: u16) -> usize {
        let before = self.entries.len();
        self.entries.retain(|stored| stored.roll.phase == phase);
        before - self.entries.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn drop_expired(&mut self, now_ms: u64) {
        self.entries
            .retain(|stored| now_ms.saturating_sub(stored.arrived_ms) < SHARED_ROLL_EXPIRY_MS);
    }
}

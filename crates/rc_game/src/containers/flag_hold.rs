use rc_proto::{ContainerFlag, StoryArray, StoryChange};

use crate::containers::{FLAG_HOLD_MS, MAX_HELD_FLAGS};

#[derive(Debug, Default)]
pub struct FlagHold {
    held: Vec<(ContainerFlag, u64)>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct HoldOutcome {
    pub fixes: Vec<StoryChange>,
    pub released: Vec<ContainerFlag>,
    pub expired: Vec<ContainerFlag>,
}

impl FlagHold {
    pub fn hold(&mut self, flags: &[ContainerFlag], now_ms: u64) {
        for flag in flags {
            if self.held.len() < MAX_HELD_FLAGS && !self.held.iter().any(|(held, _)| held == flag) {
                self.held.push((*flag, now_ms));
            }
        }
    }

    pub fn clear(&mut self) {
        self.held.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }

    pub fn filter(
        &mut self,
        fixes: Vec<StoryChange>,
        host_has_flag: impl Fn(StoryArray, u16) -> bool,
        now_ms: u64,
    ) -> HoldOutcome {
        let mut outcome = HoldOutcome::default();
        self.held.retain(|(flag, since)| {
            if host_has_flag(flag.array, flag.id) {
                outcome.released.push(*flag);
                false
            } else if now_ms.saturating_sub(*since) >= FLAG_HOLD_MS {
                outcome.expired.push(*flag);
                false
            } else {
                true
            }
        });
        outcome.fixes = fixes
            .into_iter()
            .filter(|change| !self.holds_clearing(*change))
            .collect();
        outcome
    }

    fn holds_clearing(&self, change: StoryChange) -> bool {
        let StoryChange::Flag {
            array,
            id,
            on: false,
        } = change
        else {
            return false;
        };
        self.held
            .iter()
            .any(|(flag, _)| flag.array == array && flag.id == id)
    }
}

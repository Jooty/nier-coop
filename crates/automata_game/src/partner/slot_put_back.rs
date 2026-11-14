use crate::partner::{
    BODY_BACK_IN_THE_SLOT_THIS_LONG_ENDS_A_FALL_MS, BODY_PUT_BACK_RETRY_MS,
    BODY_PUT_BACK_TRIES_PER_FALL,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutBackTurn {
    Try { number: u8 },
    Wait,
    OutOfTries,
}

/// The bounded put-backs of one fall of a body out of the hero's partner slot.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SlotPutBack {
    tries: u8,
    last_try_ms: Option<u64>,
    back_in_the_slot_since_ms: Option<u64>,
}

impl SlotPutBack {
    pub fn tries(&self) -> u8 {
        self.tries
    }

    /// Called while the body is out of the slot: a try now, a wait for the next one, or no tries left.
    pub fn turn(&mut self, now_ms: u64) -> PutBackTurn {
        self.back_in_the_slot_since_ms = None;
        if self
            .last_try_ms
            .is_some_and(|last_ms| now_ms.saturating_sub(last_ms) < BODY_PUT_BACK_RETRY_MS)
        {
            return PutBackTurn::Wait;
        }
        if self.tries >= BODY_PUT_BACK_TRIES_PER_FALL {
            return PutBackTurn::OutOfTries;
        }
        self.tries += 1;
        self.last_try_ms = Some(now_ms);
        PutBackTurn::Try { number: self.tries }
    }

    /// Called while the body is in the slot: once it has stayed there long enough the fall is over.
    pub fn note_in_the_slot(&mut self, now_ms: u64) {
        if self.tries == 0 {
            return;
        }
        let since_ms = *self.back_in_the_slot_since_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(since_ms) >= BODY_BACK_IN_THE_SLOT_THIS_LONG_ENDS_A_FALL_MS {
            *self = Self::default();
        }
    }
}

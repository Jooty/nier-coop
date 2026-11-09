use crate::fight_sync::{QUEUED_CALL_GIVEN_UP_AFTER_MS, SlotState};
use crate::story_relay::QUEUED_CALL_READY_STATE;

/// A call this hook put in the engine queue: its name and the argument block of the queue slot
/// it took. Time waited counts only while the script slot was not held (a menu).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnQueuedCall {
    pub name: String,
    pub arguments: u64,
    pub waited_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueuedCallNotice {
    /// The call in flight left the engine queue without popping; it is queued again.
    LeftQueue { name: String },
    /// The call in flight stayed queued past the limit; it is no longer waited for.
    GaveUp { name: String, waited_ms: u64 },
    /// A call of a window or epoch left behind stayed queued past the limit; it no longer blocks.
    EarlierGaveUp { name: String, waited_ms: u64 },
    RefusedPopGaveUp { name: String, waited_ms: u64 },
}

/// Our calls still in the engine queue from a window or epoch this game has left. Each one blocks
/// the next call of ours until its slot no longer holds it, it pops, or the limit passes.
#[derive(Debug, Clone, Default)]
pub struct EarlierQueuedCalls {
    calls: Vec<OwnQueuedCall>,
}

impl OwnQueuedCall {
    pub fn new(name: &str, arguments: u64) -> Self {
        Self {
            name: name.to_owned(),
            arguments,
            waited_ms: 0,
        }
    }

    pub fn is(&self, name: &str, arguments: u64) -> bool {
        self.arguments == arguments && self.name == name
    }
}

impl EarlierQueuedCalls {
    pub const fn new() -> Self {
        Self { calls: Vec::new() }
    }

    pub fn add(&mut self, call: OwnQueuedCall) {
        if !self
            .calls
            .iter()
            .any(|kept| kept.arguments == call.arguments)
        {
            self.calls.push(call);
        }
    }

    /// A queued call popped: true when it is one of these, which leaves the list.
    pub fn popped(&mut self, name: &str, arguments: u64) -> bool {
        let Some(index) = self.calls.iter().position(|call| call.is(name, arguments)) else {
            return false;
        };
        self.calls.remove(index);
        true
    }

    /// Once per tick with the time to count: drops the calls whose slot no longer holds them
    /// (`still_queued` reads the slot) and the ones past the limit.
    pub fn check(
        &mut self,
        counted_ms: u64,
        mut still_queued: impl FnMut(&str, u64) -> bool,
    ) -> Vec<QueuedCallNotice> {
        let mut notices = Vec::new();
        self.calls.retain_mut(|call| {
            call.waited_ms = call.waited_ms.saturating_add(counted_ms);
            if !still_queued(&call.name, call.arguments) {
                return false;
            }
            if call.waited_ms > QUEUED_CALL_GIVEN_UP_AFTER_MS {
                notices.push(QueuedCallNotice::EarlierGaveUp {
                    name: call.name.clone(),
                    waited_ms: call.waited_ms,
                });
                return false;
            }
            true
        });
        notices
    }

    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    pub fn len(&self) -> usize {
        self.calls.len()
    }
}

/// The milliseconds since the last check that count towards a queued call's wait: none while
/// stepping is held, so a player sitting in a menu never makes a call look lost.
pub const fn counted_wait_ms(slot: SlotState, last_check_ms: Option<u64>, now_ms: u64) -> u64 {
    match (slot, last_check_ms) {
        (SlotState::SteppingHeld, _) | (_, None) => 0,
        (_, Some(at)) => now_ms.saturating_sub(at),
    }
}

/// A queue slot still holds our call: it is ready to pop and carries the same name. A slot that
/// popped and was taken again by another call reads as someone else's.
pub fn is_our_queued_call(slot_state: Option<i32>, slot_name: Option<&str>, name: &str) -> bool {
    slot_state == Some(QUEUED_CALL_READY_STATE) && slot_name == Some(name)
}

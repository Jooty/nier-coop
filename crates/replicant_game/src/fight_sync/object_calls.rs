use crate::script_runner::QUEUE_SLOT_COUNT;

/// Who marked a queued call ready to pop (`0x413A70`, state 1 -> 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmedBy {
    /// Script code (a native inside the script step) or this hook's own relay paths.
    ScriptOrRelay,
    /// The game's own objects outside the script step: an event action the player took
    /// (`_SetEventActionCallback` queues its handler at registration and the action arms it), or a
    /// gimmick that queues and arms a call when the player comes near.
    GameObject,
}

/// The queue slots a game object armed and that have not popped yet, keyed by the slot's
/// argument block address, with the call name the slot held when it was armed.
#[derive(Debug, Clone, Default)]
pub struct ObjectArmedCalls {
    armed: Vec<(u64, String)>,
}

impl ObjectArmedCalls {
    pub const fn new() -> Self {
        Self { armed: Vec::new() }
    }

    pub fn armed(&mut self, slot_arguments: u64, name: &str, by: ArmedBy) {
        self.armed.retain(|(slot, _)| *slot != slot_arguments);
        if by == ArmedBy::ScriptOrRelay {
            return;
        }
        if self.armed.len() >= QUEUE_SLOT_COUNT as usize {
            self.armed.remove(0);
        }
        self.armed.push((slot_arguments, name.to_owned()));
    }

    /// The queued call in this slot popped: true when a game object armed it under that name.
    pub fn popped(&mut self, slot_arguments: u64, name: &str) -> bool {
        let Some(index) = self
            .armed
            .iter()
            .position(|(slot, _)| *slot == slot_arguments)
        else {
            return false;
        };
        let (_, armed_name) = self.armed.remove(index);
        armed_name == name
    }

    /// A phase restart zeroes every queue slot.
    pub fn clear(&mut self) {
        self.armed.clear();
    }

    pub fn len(&self) -> usize {
        self.armed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.armed.is_empty()
    }
}

/// The call name a queue slot holds (`QUEUED_CALL_NAME_CAPACITY` bytes from the slot's name
/// offset, NUL-terminated); `None` when empty or not text.
pub fn queued_call_name(bytes: &[u8]) -> Option<String> {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let name = core::str::from_utf8(&bytes[..end]).ok()?;
    (!name.is_empty()).then(|| name.to_owned())
}

/// A pop this game's own player caused, so inside a fight window it goes to the host as a
/// request: a fired registry entry of a kind tested against the local player, or a queued call
/// a game object armed.
pub const fn is_pop_by_this_player(
    fired_player_tested_kind: bool,
    queued_armed_by_object: bool,
) -> bool {
    fired_player_tested_kind || queued_armed_by_object
}

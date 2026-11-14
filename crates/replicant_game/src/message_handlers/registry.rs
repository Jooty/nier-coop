use crate::message_handlers::{
    MAX_MESSAGE_HANDLERS, PLAYER_LOCAL_HANDLER_NAMES, PLAYER_LOCAL_MESSAGE_IDS,
};
use crate::own_ride::is_a_ride_function;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    phase: u16,
    name: String,
    message_id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerRegistered {
    New,
    Known,
    TableFull,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageHandlers {
    entries: Vec<Entry>,
}

impl MessageHandlers {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, phase: u16, name: &str, message_id: i32) -> HandlerRegistered {
        if self.entries.iter().any(|entry| {
            entry.phase == phase && entry.name == name && entry.message_id == message_id
        }) {
            return HandlerRegistered::Known;
        }
        if self.entries.len() >= MAX_MESSAGE_HANDLERS {
            return HandlerRegistered::TableFull;
        }
        self.entries.push(Entry {
            phase,
            name: name.to_owned(),
            message_id,
        });
        HandlerRegistered::New
    }

    pub fn is_registered(&self, phase: u16, name: &str) -> bool {
        self.registrations(phase, name).next().is_some()
    }

    pub fn is_host_driven(&self, phase: u16, name: &str) -> bool {
        let mut seen = false;
        for entry in self.registrations(phase, name) {
            if is_player_local_handler(&entry.name, entry.message_id) {
                return false;
            }
            seen = true;
        }
        seen
    }

    /// `name` is registered in `phase` for a message the game keeps on each player's own game,
    /// a base one or one of `more_local_message_ids` (a fight window's additions).
    pub fn is_player_local(&self, phase: u16, name: &str, more_local_message_ids: &[i32]) -> bool {
        self.registrations(phase, name).any(|entry| {
            is_player_local_handler(&entry.name, entry.message_id)
                || more_local_message_ids.contains(&entry.message_id)
        })
    }

    fn registrations<'a>(&'a self, phase: u16, name: &'a str) -> impl Iterator<Item = &'a Entry> {
        self.entries
            .iter()
            .filter(move |entry| entry.phase == phase && entry.name == name)
    }
}

pub fn is_player_local_handler(name: &str, message_id: i32) -> bool {
    PLAYER_LOCAL_HANDLER_NAMES.contains(&name)
        || PLAYER_LOCAL_MESSAGE_IDS.contains(&message_id)
        || is_a_ride_function(name)
}

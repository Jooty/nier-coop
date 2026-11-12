use std::collections::BTreeMap;

use crate::partner::{BodySnapshot, InputFeed, RemoteInput};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InputsBySlot {
    inputs: BTreeMap<u8, RemoteInput>,
}

impl InputsBySlot {
    pub fn receive(&mut self, slot: u8, snapshot: &BodySnapshot) {
        self.inputs.entry(slot).or_default().receive(snapshot);
    }

    pub fn drop_presses(&mut self, slot: u8) {
        if let Some(input) = self.inputs.get_mut(&slot) {
            input.drop_presses();
        }
    }

    pub fn take_feed(&mut self, slot: u8) -> InputFeed {
        self.inputs
            .get_mut(&slot)
            .map_or_else(InputFeed::default, RemoteInput::take_feed)
    }

    pub fn forget(&mut self, slot: u8) {
        self.inputs.remove(&slot);
    }

    pub fn holds_input_of(&self, slot: u8) -> bool {
        self.inputs.contains_key(&slot)
    }
}

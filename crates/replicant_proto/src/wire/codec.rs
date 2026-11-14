use crate::wire::{Message, WireError};

pub fn encode(message: &Message) -> Result<Vec<u8>, WireError> {
    nt_wire::encode(message)
}

pub fn decode(bytes: &[u8]) -> Result<Message, WireError> {
    nt_wire::decode(bytes)
}

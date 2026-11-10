use crate::wire::{Message, WireError};

pub fn encode(message: &Message) -> Result<Vec<u8>, WireError> {
    nc_wire::encode(message)
}

pub fn decode(bytes: &[u8]) -> Result<Message, WireError> {
    nc_wire::decode(bytes)
}

mod slot;
mod wire;

pub use slot::PlayerSlot;
pub use wire::{
    BuildGate, Hello, MAX_ANIMATION_SECONDS, MAX_DISPLAY_NAME_BYTES, MAX_HP, MAX_MESSAGE_BYTES,
    MAX_PLAYERS, Message, PROTOCOL_VERSION, Ping, PlayerState, Pong, Reject, RejectReason, Welcome,
    WireError, decode, encode,
};

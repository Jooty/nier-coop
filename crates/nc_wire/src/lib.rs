mod build_gate;
mod codec;
mod error;
mod game_message;
mod link;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use error::WireError;
pub use game_message::GameMessage;
pub use link::{Hello, LinkPayload, Ping, Pong, Reject, RejectReason, Welcome};

pub const MAX_MESSAGE_BYTES: usize = 512;

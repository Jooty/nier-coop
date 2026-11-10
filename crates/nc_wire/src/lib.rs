mod build_gate;
mod codec;
mod error;
mod game_message;
mod game_readiness;
mod link;
mod lobby;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use error::WireError;
pub use game_message::GameMessage;
pub use game_readiness::GameReadiness;
pub use link::{Hello, LinkPayload, Ping, Pong, Reject, RejectReason, Welcome};
pub use lobby::{
    LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart, MAX_LOBBY_SLOT,
};

pub const MAX_MESSAGE_BYTES: usize = 512;

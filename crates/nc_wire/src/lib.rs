mod build_gate;
mod codec;
mod envelope;
mod error;
mod game_message;
mod game_readiness;
mod link;
mod lobby;
mod session_rules;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use envelope::{
    ADDRESSED_TO_EVERYONE, ENVELOPE_BYTES, Envelope, decode_enveloped, encode_enveloped,
};
pub use error::WireError;
pub use game_message::GameMessage;
pub use game_readiness::GameReadiness;
pub use link::{Hello, LinkPayload, Member, Ping, Pong, Reject, RejectReason, Welcome};
pub use lobby::{
    LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart, MAX_LOBBY_SLOT,
};
pub use session_rules::{MAX_SESSION_SLOTS, STORY_MODE, SessionRules};

pub const MAX_MESSAGE_BYTES: usize = 512;
pub const MAX_FRAME_BYTES: usize = MAX_MESSAGE_BYTES + ENVELOPE_BYTES;

mod build_gate;
mod codec;
mod error;
mod message;
mod motion_state;
mod player_state;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use error::WireError;
pub use message::{
    Challenge, Hello, Message, PathInfo, Ping, Pong, Proof, Reject, RejectReason, Welcome,
};
pub use motion_state::MotionState;
pub use player_state::PlayerState;

pub const PROTOCOL_VERSION: u16 = 3;
pub const MAX_MESSAGE_BYTES: usize = 256;
pub const MAX_DISPLAY_NAME_BYTES: usize = 32;
pub const MAX_PLAYERS: u8 = 2;
pub const MAX_HP: u32 = 1_000_000;
pub const MAX_POSITION_ABS: f32 = 1_000_000.0;
pub const MAX_YAW_ABS_RADIANS: f32 = 16.0;
pub const MAX_MOTION_FRAME: f32 = 100_000.0;
pub const MAX_MOTION_SPEED_ABS: f32 = 32.0;

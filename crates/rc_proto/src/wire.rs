mod build_gate;
mod codec;
mod combat_event;
mod enemy;
mod error;
mod follow_phase;
mod game_readiness;
mod message;
mod motion_state;
mod partner_activity;
mod player_state;
mod session_options;
mod weapon_state;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use combat_event::CombatEvent;
pub use enemy::{EnemyGone, EnemyGoneReason, EnemyId, EnemySnapshot, EnemySpawned, EnemyState};
pub use error::WireError;
pub use follow_phase::FollowPhase;
pub use game_readiness::GameReadiness;
pub use message::{
    Challenge, Hello, Message, PathInfo, Ping, Pong, Proof, Reject, RejectReason, Welcome,
};
pub use motion_state::MotionState;
pub use partner_activity::PartnerActivity;
pub use player_state::PlayerState;
pub use session_options::SessionOptions;
pub use weapon_state::WeaponState;

pub const PROTOCOL_VERSION: u16 = 5;
pub const MAX_MESSAGE_BYTES: usize = 256;
pub const MAX_DISPLAY_NAME_BYTES: usize = 32;
pub const MAX_PLAYERS: u8 = 2;
pub const MAX_PHASE_POINT: i32 = 99;
pub const MAX_HP: u32 = 1_000_000;
pub const MAX_POSITION_ABS: f32 = 1_000_000.0;
pub const MAX_YAW_ABS_RADIANS: f32 = 16.0;
pub const MAX_MOTION_FRAME: f32 = 100_000.0;
pub const MAX_MOTION_SPEED_ABS: f32 = 32.0;
pub const MAX_WEAPON_ID: u8 = 63;
pub const MAX_WEAPON_LEVEL: u8 = 3;
pub const ENEMY_GROUP_COUNT: u16 = 0x400;
pub const MAX_ENEMIES_PER_SNAPSHOT: usize = 5;
pub const MAX_ENEMY_SNAPSHOT_PARTS: u8 = 6;

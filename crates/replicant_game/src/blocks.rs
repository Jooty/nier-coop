mod cell;
mod glide;
mod hold_announcer;
mod outbox;
mod partner_holds;
mod placement;

pub use cell::{cell_of, distance_between};
pub use glide::{BlockGlides, GlideStep, glide_toward};
pub use hold_announcer::{HoldAnnouncement, HoldAnnouncer};
pub use outbox::BlockOutbox;
pub use partner_holds::{HoldChange, PartnerHolds};
pub use placement::{read_block_position, write_block_position};

use crate::offsets::ACTOR_TRANSFORM_BLOCK_OFFSET;

pub const PLAYER_CAN_GRAB_BLOCK_RVA: u32 = 0x006B_EB60;
pub const BLOCK_REMOTE_OWNERSHIP_MS: u64 = 1_500;
pub const BLOCK_SETTLE_MS: u64 = 1_500;
pub const BLOCK_SEND_INTERVAL_MS: u64 = 50;
pub const BLOCK_SEND_DISTANCE: f32 = 0.02;
pub const BLOCK_GLIDE_TIME_CONSTANT_MS: f32 = 60.0;
pub const BLOCK_GLIDE_LANDED_DISTANCE: f32 = 0.005;
pub const BLOCK_GLIDE_SNAP_DISTANCE: f32 = 3.0;
pub const BLOCK_GLIDE_MAX_FRAME_MS: u64 = 100;
pub const BLOCK_GLIDE_LOG_INTERVAL_MS: u64 = 1_000;
pub const BLOCK_FINAL_RESEND_DISTANCE: f32 = 0.001;
pub const BLOCK_HELD_REPEAT_INTERVAL_MS: u64 = 500;
pub const PARTNER_HOLD_EXPIRY_MS: u64 = 2_000;
pub const BLOCK_GRAB_REFUSED_LOG_INTERVAL_MS: u64 = 1_000;
pub const BLOCK_POSITION_X_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0xC;
pub const BLOCK_POSITION_Y_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0x1C;
pub const BLOCK_POSITION_Z_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0x2C;

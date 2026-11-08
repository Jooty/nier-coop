mod cell;
mod glide;
mod outbox;
mod placement;
mod roster;
mod scan;
mod state_trail;
mod text;

pub use cell::{cell_of, distance_between};
pub use glide::{BlockGlides, GlideStep, glide_toward};
pub use outbox::BlockOutbox;
pub use placement::{read_block_position, write_block_position};
pub use roster::{BlockRoster, BlockSighting, KnownBlock};
pub use scan::{block_is_live, read_block_sightings};
pub use state_trail::{StateChange, StateDiff, StateTrail, changed_words};
pub use text::{
    block_glide_start_text, block_known_text, block_landed_text, block_missing_text,
    block_moved_text, block_read_back_text, block_sent_text, held_block_text, other_map_text,
    state_diff_text,
};

use crate::offsets::ACTOR_TRANSFORM_BLOCK_OFFSET;

pub const BLOCK_VTABLE_RVA: u32 = 0x00B0_0150;
pub const BLOCK_READ_BACK_DELAY_MS: u64 = 300;
pub const BLOCK_SCAN_INTERVAL_MS: u64 = 500;
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
pub const BLOCK_STATE_SNAPSHOT_BYTES: usize = 0x400;
pub const MAX_STATE_DIFF_LINES: usize = 64;
pub const BLOCK_GRAB_REACH: f32 = 4.0;
pub const BLOCK_POSITION_X_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0xC;
pub const BLOCK_POSITION_Y_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0x1C;
pub const BLOCK_POSITION_Z_OFFSET: u64 = ACTOR_TRANSFORM_BLOCK_OFFSET + 0x2C;

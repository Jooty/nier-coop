mod grab_watch;
mod links;
mod nearby;
mod text;

pub use grab_watch::{GrabEvent, GrabWatch};
pub use links::{ActorLink, LinkKind, LinkTarget, find_links, read_block_lossy};
pub use nearby::{
    NearbyActor, ascii_runs, distance_between_points, nearest_actor_index, read_nearby_actors,
};
pub use text::{
    grab_link_text, grab_started_text, held_position_text, nearby_actor_text, released_text,
};

pub const GRAB_START_MOTION_NAME_PART: &str = "boxGrab_st";
pub const RELEASE_MOTION_NAME_PART: &str = "boxRelease";
pub const BOX_MOTION_NAME_PART: &str = "box";
pub const NEAR_ACTOR_RANGE: f32 = 4.0;
pub const HELD_POSITION_LOG_INTERVAL_MS: u64 = 500;
pub const HOLD_GIVE_UP_AFTER_MS: u64 = 120_000;
pub const NON_BOX_MOTION_GIVE_UP_AFTER_MS: u64 = 1_500;
pub const PLAYER_LINK_SCAN_BYTES: usize = 0x2000;
pub const LINK_SCAN_CHUNK_BYTES: usize = 0x400;
pub const ACTOR_TEXT_SCAN_BYTES: usize = 0x200;
pub const POINTED_TEXT_READ_BYTES: usize = 32;
pub const SMALLEST_READABLE_TEXT_LEN: usize = 5;
pub const LOWEST_PLAUSIBLE_POINTER: u64 = 0x1_0000;
pub const HIGHEST_PLAUSIBLE_POINTER: u64 = 0x7FFF_FFFF_FFFF;
pub const MAX_NEARBY_ACTORS_LOGGED: usize = 24;

mod broken_walls;
mod scan;
mod walls;

pub use broken_walls::{BrokenWalls, same_wall};
pub use scan::{SightedBreakable, nearest_breakable, pool_actors, read_breakables};
pub use walls::{
    BreakableClass, BreakableState, bomb_break_record, break_is_shared, break_started,
    breakable_class,
};

pub const BREAKABLE_BOX_VTABLE_RVA: u32 = 0x00B0_4230;
pub const SECOND_BREAKABLE_BOX_VTABLE_RVA: u32 = 0x00B0_4B70;
pub const BREAKABLE_RECEIVE_HIT_FUNCTION_RVA: u32 = 0x0016_8790;
pub const BREAKABLE_BOMB_ONLY_MODE_OFFSET: u64 = 0x1_4DE8;
pub const BREAKABLE_BREAK_TIMER_OFFSET: u64 = 0x1_4E00;
pub const BREAKABLE_HITS_LEFT_OFFSET: u64 = 0x1_4E04;
pub const BREAKABLE_BOMB_ONLY_MODE: i32 = 1;
pub const BREAKABLE_HITS_LEFT_FOR_ONE_LAST_HIT: i32 = 1;
pub const BOMB_BLAST_HIT_FLAG: u32 = 0x8000;
pub const BOMB_BLAST_ATTACK_FLAGS: u32 = 0x8080;
pub const ATTACK_DIRECTION_GIVEN_FLAG: u32 = 0x20;
pub const SYNTHETIC_ATTACK_RECORD_BYTES: usize = 0x120;
pub const WALL_MATCH_DISTANCE: f32 = 2.0;
pub const SAME_WALL_DISTANCE: f32 = 0.5;
pub const WALL_RETRY_INTERVAL_MS: u64 = 500;
pub const WALL_GIVE_UP_MS: u64 = 15_000;
pub const MAX_PENDING_WALLS: usize = 16;

mod census;
mod enemy_id;
mod enemy_messages;
mod enemy_share;
mod health_share;
mod layout_read;
mod roster_exchange;

pub use census::{Census, MismatchWatch, RosterMismatch, distinct_sightings, roster_mismatches};
pub use enemy_id::{EnemyId, EnemySighting};
pub use enemy_messages::{EnemyHit, EnemyHits, EnemyRosterPart};
pub use enemy_share::EnemyShare;
pub use health_share::{
    DefeatWatch, GuestHealthStep, HitLedger, HitOwner, HitSettlement, amount_in_the_hosts_scale,
    guest_health_step, health_in_the_guests_scale, hit_owner, settle_hit,
};
pub use layout_read::{
    LayoutsHeader, Placement, enemy_placements, is_enemy_object_id, layouts_header,
    reads_as_an_enemys_health,
};
pub use roster_exchange::{RosterAssembly, RosterPacer, roster_parts};

pub const ENTITY_LAYOUTS_HEADER_LEN: usize = 0x88;
pub const ENTITY_LAYOUTS_ITEMS_POINTER_OFFSET: usize = 0x30;
pub const ENTITY_LAYOUTS_ITEM_COUNT_OFFSET: usize = 0x38;
pub const ENTITY_LAYOUTS_ID_OFFSET: usize = 0x50;
pub const ENTITY_LAYOUTS_NEXT_POINTER_OFFSET: usize = 0x78;
pub const ENTITY_LAYOUT_LEN: usize = 0x90;
pub const ENTITY_LAYOUT_OBJECT_ID_OFFSET: usize = 0x20;
pub const ENTITY_LAYOUT_ENTITY_HANDLE_OFFSET: usize = 0x70;
pub const ENTITY_LAYOUT_ID_OFFSET: usize = 0x80;
pub const MOST_ENTITY_LAYOUTS_LISTS_WALKED: usize = 2_048;
pub const MOST_ENTITY_LAYOUTS_READ_OF_ONE_LIST: u64 = 1_024;
pub const EMPTY_ENTITY_HANDLES: [u32; 2] = [0, u32::MAX];

pub const LOWEST_ENEMY_OBJECT_ID: u32 = 0x2_0000;
pub const HIGHEST_ENEMY_OBJECT_ID: u32 = 0x2_FFFF;
pub const ENTITY_BEHAVIOR_POINTER_OFFSET: u32 = 0x48;
pub const ENEMY_MAX_HEALTH_OFFSET: u32 = 0x85C;
pub const ENEMY_PLACEMENT_ID_OFFSET: u32 = 0x66C;
pub const ENEMY_DAMAGE_VTABLE_OFFSET: u32 = 0x4C0;
pub const ENEMY_FLAT_DAMAGE_VTABLE_OFFSET: u32 = 0x4C8;
pub const HIT_RECORD_ATTACKER_HANDLE_OFFSET: u32 = 0x68;
pub const ENEMY_FLAGS_THE_FORCED_DEFEAT_SETS_OFFSETS: [u32; 2] = [0x2_8040, 0x2_8044];
pub const ENEMY_FORCED_DEFEAT_STATE: i32 = 0x8E;
pub const ENEMY_FORCED_DEFEAT_STATE_WHEN_THE_GAMES_TEST_SAYS_SO: i32 = 0xAC;
pub const ENEMY_DAMAGE_FUNCTION_START: [u8; 32] = [
    0x48, 0x8B, 0xC4, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x83, 0xEC, 0x70,
    0x48, 0xC7, 0x40, 0x98, 0xFE, 0xFF, 0xFF, 0xFF, 0x48, 0x89, 0x58, 0x08, 0x48, 0x89, 0x68, 0x10,
];
pub const ENEMY_FLAT_DAMAGE_FUNCTION_START: [u8; 15] = [
    0x48, 0x89, 0x5C, 0x24, 0x20, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0xFA, 0x48, 0x8B, 0xD9,
];
pub const ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION_START: [u8; 32] = [
    0xF7, 0x81, 0x98, 0x05, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x0F, 0x85, 0xD2, 0x00, 0x00, 0x00,
    0x8B, 0x89, 0x9C, 0x81, 0x02, 0x00, 0x8D, 0x81, 0xF0, 0xFF, 0xFD, 0xFF, 0xA9, 0xBF, 0xFF, 0xFF,
];

pub const ENEMY_CENSUS_INTERVAL_MS: u64 = 100;
pub const ENEMIES_IN_ONE_ROSTER_PART: usize = 16;
pub const MOST_ENEMIES_IN_ONE_ROSTER: usize = 128;
pub const ENEMY_HITS_IN_ONE_MESSAGE: usize = 16;
pub const UNCHANGED_ROSTER_IS_SENT_AGAIN_AFTER_MS: u64 = 1_000;
pub const CHANGED_ROSTER_IS_SENT_AFTER_MS: u64 = 100;
pub const ROSTER_MISMATCH_MUST_LAST_MS: u64 = 2_000;
pub const HOST_HEALTH_ABOVE_THE_LOCAL_ONE_WAITS_AFTER_A_LOCAL_HIT_MS: u64 = 500;
pub const CENSUS_LINES_LOGGED_AT_MOST: u32 = 400;
pub const MOST_ENEMIES_NAMED_ONE_BY_ONE_IN_ONE_CENSUS: usize = 6;
pub const MOST_DEFEATED_ENEMIES_REMEMBERED: usize = 256;
pub const HIGHEST_ENEMY_HEALTH_ON_THE_WIRE: i32 = 100_000_000;

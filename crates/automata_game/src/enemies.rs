mod census;
mod enemy_id;
mod enemy_messages;
mod enemy_share;
mod generator_census;
mod generator_read;
mod guest_spawns;
mod health_share;
mod host_spawns;
mod layout_read;
mod roster_exchange;
mod spawn_messages;

pub use census::{Census, MismatchWatch, RosterMismatch, distinct_sightings, roster_mismatches};
pub use enemy_id::{EnemyId, EnemySighting};
pub use enemy_messages::{EnemyHit, EnemyHits, EnemyRosterPart};
pub use enemy_share::EnemyShare;
pub use generator_census::{GeneratorCensus, generators_with_a_shared_id};
pub use generator_read::{
    GeneratorHeader, GeneratorPhase, GeneratorSlot, generator_header, generator_slots,
    new_slot_bytes, object_id_of_set, spawn_block_bytes, spawn_values_of_set,
};
pub use guest_spawns::{GuestGeneratorStep, GuestGenerators, SlotRemoval};
pub use health_share::{
    DefeatWatch, GuestHealthStep, HitLedger, HitOwner, HitSettlement, amount_in_the_hosts_scale,
    guest_health_step, health_in_the_guests_scale, hit_owner, settle_hit,
};
pub use host_spawns::{HostSpawns, ObservedSpawn};
pub use layout_read::{
    LayoutsHeader, Placement, enemy_placements, is_enemy_object_id, layouts_header,
    live_entity_slots, reads_as_an_enemys_health,
};
pub use roster_exchange::{RosterAssembly, RosterPacer, roster_parts};
pub use spawn_messages::{EnemySpawn, GeneratorState, GeneratorStates};

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

pub const ENTITY_TABLE_SLOT_LEN: usize = 0x10;
pub const ENTITY_TABLE_SLOT_ENTITY_POINTER_OFFSET: usize = 0x8;
pub const MOST_ENTITY_TABLE_SLOTS_READ: u32 = 0x4000;
pub const ENTITY_NAME_OFFSET: u32 = 0x8;
pub const ENTITY_NAME_LEN: usize = 0x20;
pub const ENEMY_SET_WORD_OFFSET: u32 = 0x668;
pub const ENTITY_TABLE_IS_READ_EVERY_MS: u64 = 1_000;
pub const MOST_UNPLACED_ENEMIES_NAMED_IN_ONE_LINE: usize = 8;

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

pub const GENERATOR_HEADER_LEN: usize = 0x400;
pub const GENERATOR_ACTION_ID_OFFSET: usize = 0x18;
pub const GENERATOR_STATE_HASH_OFFSET: usize = 0x20;
pub const GENERATOR_SPAWNER_OFFSET: usize = 0x48;
pub const GENERATOR_SPAWNED_COUNT_OFFSET: usize = 0x48;
pub const GENERATOR_SLOT_ARRAY_OFFSET: usize = 0x58;
pub const GENERATOR_SLOTS_POINTER_OFFSET: usize = 0x60;
pub const GENERATOR_SLOT_COUNT_OFFSET: usize = 0x68;
pub const GENERATOR_ENEMIES_OF_ONE_RUN_OFFSET: usize = 0x80;
pub const GENERATOR_MOST_ENEMIES_ALIVE_OFFSET: usize = 0x84;
pub const GENERATOR_SETS_POINTER_OFFSET: usize = 0x158;
pub const GENERATOR_SET_COUNT_OFFSET: usize = 0x160;
pub const GENERATOR_SCREEN_TEST_KIND_OFFSET: usize = 0x198;
pub const GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET: usize = 0x3D1;
pub const GENERATOR_NEXT_POINTER_OFFSET: usize = 0x3F8;
pub const GENERATOR_SLOT_ARRAY_ADD_VTABLE_OFFSET: usize = 0x10;
pub const GENERATOR_SLOT_LEN: usize = 0x40;
pub const GENERATOR_SLOT_STATE_OFFSET: usize = 0x0;
pub const GENERATOR_SLOT_SET_INDEX_OFFSET: usize = 0x4;
pub const GENERATOR_SLOT_ENTITY_HANDLE_OFFSET: usize = 0x8;
pub const GENERATOR_SLOT_POSITION_OFFSET: usize = 0x20;
pub const GENERATOR_SLOT_ROTATION_OFFSET: usize = 0x30;
pub const GENERATOR_SLOT_STATE_NEW: i32 = 0;
pub const GENERATOR_SLOT_STATE_ENEMY_MADE: i32 = 3;
pub const GENERATOR_SLOT_STATE_FREED: i32 = 5;
pub const GENERATOR_SLOT_STATE_ENEMY_NOT_MADE: i32 = 6;
pub const GENERATOR_SET_LEN: usize = 0x30;
pub const GENERATOR_SET_OBJECT_ID_OFFSET: usize = 0x0;
pub const FIRST_OF_THE_FIRST_SET_VALUES: usize = 1;
pub const SET_WORD_HANDED_ON_AS_IT_IS: usize = 4;
pub const SET_WORD_THAT_SAYS_SECOND_VALUES_EXIST: usize = 5;
pub const FIRST_OF_THE_SECOND_SET_VALUES: usize = 6;
pub const SPAWN_BLOCK_LEN: usize = 0x90;
pub const SPAWN_BLOCK_POSITION_OFFSET: usize = 0x40;
pub const SPAWN_BLOCK_ROTATION_OFFSET: usize = 0x50;
pub const SPAWN_BLOCK_SCALE_OFFSET: usize = 0x60;
pub const SPAWN_BLOCK_SET_VALUES_OFFSET: usize = 0x70;
pub const SPAWN_WITHOUT_A_GENERATOR_WAITS_MS: u64 = 3_000;
pub const SPAWN_WITHOUT_A_GENERATOR_IS_MADE_WITHIN_METRES: f32 = 200.0;
pub const GENERATOR_STATE_HASH_WAITING: u32 = 0x4B7B_6E51;
pub const GENERATOR_STATE_HASH_SPAWNING: u32 = 0xE361_D0B2;
pub const GENERATOR_STATE_HASH_RESTING: u32 = 0xAF40_F7B2;
pub const GENERATOR_STATE_HASH_FINISHED: u32 = 0x26B8_0D1F;
pub const GENERATOR_STATE_HASH_SWITCHED_OFF: u32 = 0xE742_EEB9;
pub const MOST_GENERATORS_WALKED: usize = 4_096;
pub const MOST_SLOTS_READ_OF_ONE_GENERATOR: u64 = 256;
pub const MOST_SETS_OF_ONE_GENERATOR: u32 = 256;
pub const GENERATORS_ARE_READ_EVERY_MS: u64 = 1_000;
pub const GENERATOR_STATES_IN_ONE_MESSAGE: usize = 16;
pub const MOST_GENERATOR_STATES_TOLD: usize = 256;
pub const MOST_HOST_SPAWNS_HELD: usize = 512;
pub const HIGHEST_SPAWN_COORDINATE_ABS: f32 = 1_000_000.0;
pub const LOWEST_ENEMY_LEVEL_ON_THE_WIRE: i32 = -32_768;
pub const HIGHEST_ENEMY_LEVEL_ON_THE_WIRE: i32 = 32_767;
pub const GENERATOR_LINES_LOGGED_AT_MOST: u32 = 400;
pub const MOST_GENERATORS_NAMED_IN_ONE_LINE: usize = 6;
pub const MOST_GENERATOR_IDS_IN_ONE_LINE: usize = 160;
pub const GENERATOR_SPAWNER_UPDATE_FUNCTION_START: [u8; 32] = [
    0x48, 0x8B, 0xC4, 0x55, 0x56, 0x48, 0x81, 0xEC, 0xF8, 0x00, 0x00, 0x00, 0x48, 0x89, 0x58, 0x08,
    0x48, 0x8B, 0xEA, 0x48, 0x89, 0x78, 0x10, 0x48, 0x8B, 0xF1, 0x4C, 0x89, 0x70, 0x18, 0x4C, 0x89,
];
pub const GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_MAIN_EXE: [u8; 24] = [
    0x48, 0x89, 0x5C, 0x24, 0x18, 0x55, 0x56, 0x57, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x6C, 0x24,
    0xC9, 0x48, 0x81, 0xEC, 0xF0, 0x00, 0x00, 0x00,
];
pub const GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_COMPAT_EXE: [u8; 32] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x48, 0x89, 0x7C, 0x24, 0x18, 0x55,
    0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0x6C, 0x24, 0xB9, 0x48, 0x81, 0xEC, 0xE0, 0x00, 0x00, 0x00,
];
pub const GENERATOR_LEVEL_PICK_FUNCTION_START: [u8; 29] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x80, 0xB9, 0xD0, 0x03, 0x00, 0x00,
    0x00, 0x48, 0x8B, 0xDA, 0x48, 0x8B, 0xF9, 0x0F, 0x84, 0xC7, 0x00, 0x00, 0x00,
];
pub const GENERATOR_SLOT_FREE_FUNCTION_START: [u8; 28] = [
    0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x8B, 0x01, 0x48, 0x8B, 0xD9, 0xFF, 0xC8, 0x83, 0xF8, 0x02,
    0x77, 0x13, 0x44, 0x8B, 0x41, 0x10, 0x8B, 0x51, 0x0C, 0x48, 0x8D, 0x0D,
];
pub const GENERATOR_SLOT_ADD_FUNCTION_START: [u8; 32] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0x41, 0x10, 0x48, 0x8B,
    0xFA, 0x48, 0x8B, 0x51, 0x18, 0x48, 0x8B, 0xD9, 0x48, 0x3B, 0xC2, 0x77, 0x4D, 0x48, 0x85, 0xD2,
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

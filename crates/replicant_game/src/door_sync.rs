mod mapping;
mod pending;
mod plan;
mod registry;
mod schedule;

pub use mapping::{action_of_message, message_of_action};
pub use pending::{PendingDoorStates, RetryOutcome};
pub use plan::{
    changes_for_resulting_state, door_reached_state, plan_door_state, sender_phase_matches_local,
};
pub use registry::{DoorRegistry, GuestUseWindow};
pub use schedule::SnapshotSchedule;

pub const DOOR_LOCK_MESSAGE_ID: u32 = 0x2A;
pub const DOOR_ACTIVE_MESSAGE_ID: u32 = 0x2C;
pub const DOOR_OPEN_MESSAGE_ID: u32 = 0x2D;
pub const DOOR_CLOSE_MESSAGE_ID: u32 = 0x2E;
pub const DOOR_OPEN_BYTE_OFFSET: u64 = 0x1_4D8C;
pub const DOOR_LOCK_BYTE_OFFSET: u64 = 0x1_4D94;
pub const MAP_ACTOR_BY_LAYOUT_ID_FUNCTION_RVA: u32 = 0x41_8E10;
pub const MAP_ACTOR_TABLE_POINTER_OFFSET: u64 = 0x1D_77B8;
pub const DOOR_VTABLE_RVA: u32 = 0xB0_8C50;
pub const MAX_REGISTERED_DOORS: usize = 512;
pub const GUEST_USE_WINDOW_MS: u64 = 4000;
pub const SNAPSHOT_SETTLE_MS: u64 = 3000;
pub const SNAPSHOT_APPLY_GIVE_UP_MS: u64 = 15_000;
pub const SNAPSHOT_APPLY_RETRY_MS: u64 = 500;

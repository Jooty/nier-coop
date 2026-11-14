mod cube;
mod held;
mod key;
mod kinds;
mod roster;
mod scan;
mod text;
mod verdict;

pub use cube::cube_is_hittable;
pub use held::{HeldTarget, HoldHow, box_move_handle_in_running_state, read_held_target};
pub use key::{key_text, world_object_key};
pub use kinds::{ObjectSync, WORLD_OBJECT_KINDS, WorldObjectKind, kind_of_vtable_rva};
pub use roster::{KnownObject, ObjectSighting, WorldObjectRoster};
pub use scan::{handle_owner, object_is_live, read_object_sightings};
pub use text::{
    HandleOwner, object_known_text, partner_break_text, partner_let_go_text, partner_moves_text,
    partner_took_text, refused_guest_move_text, unlisted_hold_text, you_broke_text,
    you_let_go_text, you_move_text, you_took_text,
};
pub use verdict::{BreakVerdict, MoveVerdict, partner_break_verdict, partner_move_verdict};

pub const PLAYER_REQUESTED_ACTION_STATE_OFFSET: u64 = 0x1_6208;
pub const BOX_MOVE_STATE_VTABLE_RVA: u32 = 0x00C9_4060;
pub const PICK_MOVE_STATE_VTABLE_RVA: u32 = 0x00C9_40F0;
pub const ACTION_STATE_TARGET_HANDLE_OFFSET: u64 = 0x18;
pub const CUBE_BREAK_FUNCTION_RVA: u32 = 0x0016_16E0;
pub const CUBE_HITTABLE_FLAG_OFFSET: u64 = 0x13E5;
pub const WORLD_OBJECT_SCAN_INTERVAL_MS: u64 = 500;
pub const CARRY_LOG_INTERVAL_MS: u64 = 1_000;
pub const PARTNER_BREAK_RETRY_FOR_MS: u64 = 5_000;
pub const MAX_PENDING_PARTNER_BREAKS: usize = 64;
pub const UNGROUPED_OBJECT_GROUP: u16 = 0x3FF;
pub const REFUSAL_ANSWER_INTERVAL_MS: u64 = 200;
pub const HOST_RESENDS_MOVED_OBJECTS_EVERY_MS: u64 = 2_000;

mod hold;
mod mirror;
mod plan;
mod roster;
mod scan;
mod sighting;
mod snapshot;

pub use hold::{NpcHoldBlocker, read_npc_hold_blocker};
pub use mirror::{MirroredNpc, NpcMirror};
pub use plan::{NpcPlan, NpcPlanner};
pub use roster::{NpcRoster, NpcRosterUpdate};
pub use scan::scan_npcs;
pub use sighting::NpcSighting;
pub use snapshot::{npc_state, split_npc_snapshot};

pub const NPC_SEND_INTERVAL_MS: u64 = 100;
pub const NPC_MIRROR_DELAY_MS: u64 = 200;
pub const NPC_MIRROR_STALE_MS: u64 = 1500;
pub const NPC_LOCAL_ONLY_GRACE_MS: u64 = 3000;

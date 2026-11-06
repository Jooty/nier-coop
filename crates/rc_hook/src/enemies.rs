mod combat;
mod combat_view;
mod counters;
mod drop_hook;
mod game_calls;
mod guest_combat;
mod hit_hook;
mod hold_hook;
mod host_combat;
mod install;
mod status;
mod sync;

pub use counters::CombatCounts;
pub use install::install;
pub use status::{
    EnemySyncRole, EnemySyncStatus, enemy_id_text, enemy_sync_status, set_snap_enabled,
    snap_enabled,
};
pub use sync::tick_enemy_sync;

const MAX_RECENT_ENEMY_EVENTS: usize = 6;
const COPY_KILL_ATTEMPTS: u32 = 3;
const MOTION_FRAME_MOVED: f32 = 0.01;
const MOTION_STALL_MS: u64 = 1000;
const MOTION_RESTART_INTERVAL_MS: u64 = 500;
const SECOND_MOTION_LAYER: u32 = 1;
const PENDING_KILL_MAX_AGE_MS: u64 = 3000;

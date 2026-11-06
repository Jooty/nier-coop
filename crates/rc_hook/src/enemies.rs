mod game_calls;
mod status;
mod sync;

pub use status::{
    EnemySyncRole, EnemySyncStatus, enemy_id_text, enemy_sync_status, set_snap_enabled,
    snap_enabled,
};
pub use sync::tick_enemy_sync;

const MAX_RECENT_ENEMY_EVENTS: usize = 6;

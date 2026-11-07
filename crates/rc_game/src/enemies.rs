mod aggro;
mod attack_record;
mod difficulty;
mod enemy_log;
mod ghosts;
mod guest_only;
mod hit_guard;
mod kill_watch;
mod list_diff;
mod matching;
mod mirror;
mod partners;
mod player_hit_guard;
mod removal_cause;
mod roster;
mod scan;
mod shadow_matrix;
mod sighting;
mod snapshot;
mod time_scale;

pub use aggro::{AggroPlanner, AggroPlayers, AggroTarget, choose_target};
pub use attack_record::{ActorHandle, AttackRecord};
pub use difficulty::{
    DamageTakenLedger, EXTRA_PLAYERS_WITH_ONE_GUEST, ScaledEnemyKey, damage_dealt_bonus_percent,
    damage_taken_factor, scaled_hp_damage,
};
pub use enemy_log::{EnemyLog, relayed_spawn_text, sighting_text};
pub use ghosts::{GhostPlan, plan_ghosts};
pub use guest_only::GuestOnlyWatch;
pub use hit_guard::{GuestHitGuard, HitVerdict};
pub use kill_watch::KillWatch;
pub use list_diff::{ListDiff, ListDiffWatch, LocalKeep};
pub use matching::{EnemyMatch, match_enemies};
pub use mirror::{EnemyMirror, MirroredEnemy};
pub use partners::partner_actors;
pub use player_hit_guard::PlayerHitGuard;
pub use removal_cause::RemovalCause;
pub use roster::{EnemyRoster, RosterUpdate};
pub use scan::scan_enemies;
pub use shadow_matrix::shadow_matrix;
pub use sighting::EnemySighting;
pub use snapshot::{enemy_state, split_snapshot};
pub use time_scale::{NORMAL_TIME_SCALE, restored_time_scale};

pub const ENEMY_MIRROR_STALE_MS: u64 = 1000;
pub const ENEMY_MIRROR_DELAY_MS: u64 = 100;
pub const AGGRO_PUSH_RANGE: f32 = 15.0;
pub const AGGRO_SWITCH_MARGIN: f32 = 1.5;
pub const AGGRO_REFRESH_MS: u64 = 2000;
pub const AGGRO_PUSH_WEIGHT: f32 = 100.0;
pub const GUEST_HIT_REPEAT_WINDOW_MS: u64 = 2000;
pub const PLAYER_HIT_REPEAT_WINDOW_MS: u64 = 2000;
pub const GUEST_ONLY_ENEMY_GRACE_MS: u64 = 3000;
pub const ENEMY_LIST_DIFF_INTERVAL_MS: u64 = 5000;

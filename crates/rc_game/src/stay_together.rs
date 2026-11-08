mod announce;
mod follow;
mod go_to_partner;
mod leash;
mod phase_reads;
mod spot;
mod teleport_check;

pub use announce::{AnnounceConditions, HostAnnouncer};
pub use follow::{FollowConditions, FollowRole, FollowState, FollowStep, PhaseTarget, WaitReason};
pub use go_to_partner::{GoToPartnerRefusal, check_partner_reachable};
pub use leash::{Leash, distance_between};
pub use phase_reads::{announced_target, local_phase_index};
pub use spot::{FloorHit, SpotRefusal, WorldProbe, choose_spot};
pub use teleport_check::{
    TELEPORT_ARRIVED_UNITS, TELEPORT_GIVE_UP_MS, TeleportCheck, check_teleport,
};

pub const FOLLOW_RETRY_MS: u64 = 1500;
pub const TOGETHER_FREE_TO_ACT_GRACE_MS: u64 = 600;
pub const ANNOUNCE_RESEND_MS: u64 = 2000;
pub const DEFAULT_LEASH_UNITS: f32 = 40.0;
pub const MIN_LEASH_UNITS: f32 = 5.0;
pub const MAX_LEASH_UNITS: f32 = 500.0;
pub const PARTNER_TIMEOUT_MS: u64 = 3000;
pub const SPOT_RING_RADII: [f32; 3] = [1.5, 2.5, 3.5];
pub const SPOT_ANGLE_STEPS: u32 = 8;
pub const MAX_FLOOR_STEP_FROM_PARTNER: f32 = 1.0;
pub const MIN_FLOOR_NORMAL_Y: f32 = 0.7;
pub const FLOOR_RAY_START_ABOVE_PARTNER: f32 = 2.0;
pub const FLOOR_RAY_END_BELOW_PARTNER: f32 = 6.0;
pub const BODY_RADIUS: f32 = 0.7;
pub const BODY_FLOOR_CLEARANCE: f32 = 0.1;

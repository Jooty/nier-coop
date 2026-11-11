mod actor_liveness;
mod affine;
mod attach_point;
mod body;
mod body_announce;
mod body_pack;
mod book_announce;
mod book_look;
mod draw_report;
mod ghost_book;
mod idle_pick;
mod layer_motion;
mod motion_follow;
mod own_seed;
mod pose;
mod respawn_gate;
mod retire_plan;
mod retry_gate;
mod ridden_boar;
mod weapon_look;

pub use actor_liveness::{actor_is_live, actor_of_handle};
pub use affine::Affine;
pub use attach_point::{AttachPoint, read_attach_point};
pub use body::{BODY_TABLE, BodyInfo, body_for_pack, body_for_seed, body_info};
pub use body_announce::{BODY_RESEND_INTERVAL_MS, BodyAnnouncer};
pub use body_pack::{
    BodyPackLoad, BodyPackStep, PACK_REQUEST_RETRY_MS, PACK_SETTLE_AFTER_FOUND_MS,
    release_requested_pack_wanted,
};
pub use book_announce::BookAnnouncer;
pub use book_look::{
    BOOK_BOB_PERIOD_MS, BOOK_HOVER_AMPLITUDE, book_class_id, book_hover_offset, book_pack_name,
};
pub use draw_report::{ActorDrawReport, distance_between, read_actor_draw_report};
pub use ghost_book::{GhostBook, SpawnResult};
pub use idle_pick::{IDLE_NAME_FALLBACK_PART, idle_index_from_table};
pub use layer_motion::{
    LayerMotion, MotionEntry, PuppetLayer, motion_entries, motion_entry_for_hash, motion_name,
    read_layer_motion, read_puppet_layer, seek_layer_frame,
};
pub use motion_follow::{MotionStep, follow_step, frame_to_seek, wrapped_frame};
pub use own_seed::{OwnSeedSwap, SEED_NAME_TABLE_STRIDE, own_seed_swap};
pub use pose::PuppetPose;
pub use respawn_gate::{
    BOOK_PACK_REREQUEST_TIMEOUT_MS, BOOK_SPAWN_FIRST_RETRY_MS, BOOK_SPAWN_LOG_EVERY_ATTEMPTS,
    BOOK_SPAWN_LONGEST_RETRY_MS, BookSpawnPlanner, BookSpawnStep, PUPPET_MODEL_RESPAWN_INTERVAL_MS,
    RespawnGate,
};
pub use retire_plan::{RetirePlan, retire_plan};
pub use retry_gate::RetryGate;
pub use ridden_boar::{
    BoarPackSettle, BoarStep, RIDDEN_BOAR_PACK_LOOKUP_INTERVAL_MS, RIDDEN_BOAR_PACK_SETTLE_MS,
    boar_pose_under_rider, boar_seen_from_rider, boar_step, read_ridden_boar,
    read_ridden_boar_actor,
};
pub use weapon_look::{read_weapon_grip, read_weapon_look};

pub const MOTION_SEEK_TOLERANCE_FRAMES: f32 = 0.5;
pub const MOTION_BLEND_SECONDS: f32 = 0.1;

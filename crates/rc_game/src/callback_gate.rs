mod end_follow;
mod gate;
mod names;
mod partner_copies;
mod ran_ends;

pub use end_follow::{
    EndArrival, EndDrop, EndFollow, PendingEnd, WaitStep, end_follow, is_end_callback,
    is_mirrored_callback, wait_step,
};
pub use gate::{CallbackGate, GateRole, GateStep, LocalFire, PartnerFire};
pub use names::{is_gated_callback, is_scene_talk};
pub use partner_copies::{PartnerStartedCopies, event_base};
pub use ran_ends::RanEnds;

pub const GATED_CALLBACK_NAMES: [&str; 1] = ["Boss_Battle_Start"];
pub const GATED_EVENT_SUFFIX: &str = "_Start";
pub const CALLBACK_GATE_PAIRING_WINDOW_MS: u64 = 20_000;
pub const CALLBACK_GATE_HOST_GRACE_MS: u64 = 3_000;
pub const CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS: u64 = 600;
pub const CALLBACK_GATE_GUEST_GIVE_UP_MS: u64 = 20_000;
pub const SCENE_TALK_PREFIX: &str = "SEID_";
pub const END_FOLLOW_EVENT_SUFFIX: &str = "_End";
pub const END_FOLLOW_WAIT_CAP_MS: u64 = 60_000;
pub const STARTED_FROM_PARTNER_EXPIRY_MS: u64 = 300_000;
pub const TALK_START_AFTER_COMMAND_MS: u64 = 2_000;

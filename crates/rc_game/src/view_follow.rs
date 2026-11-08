mod chains;
mod decision;
mod scene_watch;

pub use chains::{DoorChainEvent, ViewChain, door_chain_event, view_chain};
pub use decision::{DoorView, ViewAnnouncer, ViewFollow, decide_view_follow};
pub use scene_watch::SceneWatch;

pub const DOOR_IN_PREFIX: &str = "GM_IN_Door_";
pub const DOOR_OUT_PREFIX: &str = "GM_OUT_Door_";
pub const DOOR_IN_VIEW_PART_NUMBERS: [&str; 2] = ["02", "03"];
pub const VIEW_RESEND_MS: u64 = 3_000;
pub const SCENE_TELEPORT_WINDOW_MS: u64 = 30_000;
pub const SCENE_TELEPORT_JUMP_UNITS: f32 = 20.0;

mod chains;
mod decision;
mod inside_state;
mod retry;
mod scene_watch;

pub use chains::{DoorChainEvent, VIEW_CHAINS, ViewChain, door_chain_event, view_chain};
pub use decision::{DoorView, ViewAnnouncer, ViewFollow, decide_view_follow};
pub use inside_state::{view_after_callback, view_after_phase_check};
pub use retry::{RanReport, RetryAction, ViewRetry};
pub use scene_watch::SceneWatch;

pub const DOOR_IN_PREFIX: &str = "GM_IN_Door_";
pub const DOOR_OUT_PREFIX: &str = "GM_OUT_Door_";
pub const DOOR_IN_VIEW_PART_NUMBERS: [&str; 2] = ["02", "03"];
pub const VIEW_RESEND_MS: u64 = 3_000;
pub const SCENE_TELEPORT_WINDOW_MS: u64 = 30_000;
pub const SCENE_TELEPORT_JUMP_UNITS: f32 = 20.0;
pub const SIDE_VIEW_PREFIX: &str = "CB_SideView_";
pub const SIDE_VIEW_ENV_ON: &str = "SideView_ENV_ON";
pub const SIDE_VIEW_ENV_OFF: &str = "SideView_ENV_OFF";
pub const VIEW_RETRY_INTERVAL_MS: u64 = 2_000;
pub const VIEW_RETRY_CAP_MS: u64 = 30_000;
pub const VIEW_SCENE_WAIT_CAP_MS: u64 = 45_000;
pub const VIEW_SCENE_OVER_FALLBACK_MS: u64 = 8_000;

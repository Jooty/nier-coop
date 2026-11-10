mod coming_over;
mod injected;
mod recent;
mod watch;

pub use coming_over::{AnnounceDrop, AnnounceVerdict, ComingOver, ComingStep};
pub use injected::InjectedCallbacks;
pub use recent::RecentCallbacks;
pub use watch::{FollowStep, SceneFollowWatch};

pub const SCENE_FOLLOW_LOOKBACK_MS: u64 = 3_000;
pub const SCENE_FOLLOW_GRACE_MS: u64 = 3_000;
pub const SCENE_FOLLOW_ALREADY_ENTERED_MS: u64 = 2_000;
pub const SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS: u64 = 600;
pub const SCENE_FOLLOW_GIVE_UP_MS: u64 = 15_000;
pub const SCENE_FOLLOW_DUPLICATE_WINDOW_MS: u64 = 20_000;
pub const SCENE_FOLLOW_RECENT_CAPACITY: usize = 16;
pub const SCENE_FOLLOW_NAMES_SENT: usize = 1;

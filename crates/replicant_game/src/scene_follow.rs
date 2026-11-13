mod coming_over;
mod host_view;
mod injected;
mod recent;
mod watch;

pub use coming_over::{AnnounceDrop, AnnounceVerdict, ComingOver, ComingStep};
pub use host_view::{
    HostViewFacts, HostViewHold, HostViewKind, HostViewRelease, HostViewVerdict,
    guest_to_host_units,
};
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
pub const GUEST_TAKES_HOST_VIEW_WITHIN_UNITS: f32 = 20.0;
pub const HOST_VIEW_RELEASED_AFTER_GUEST_FAR_MS: u64 = 1_000;

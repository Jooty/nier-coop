mod coming_over;
mod cut;
mod injected;
mod recent;
mod unplayed;
mod watch;

pub use coming_over::{AnnounceDrop, AnnounceVerdict, ComingOver, ComingStep};
pub use cut::{LocalCut, cut_load_call, cut_name_from_buffer, cut_to_load, host_scene_cut};
pub use injected::InjectedCallbacks;
pub use recent::RecentCallbacks;
pub use unplayed::UnplayedStarts;
pub use watch::{FollowStep, SceneFollowWatch};

pub const SCENE_FOLLOW_LOOKBACK_MS: u64 = 3_000;
pub const SCENE_FOLLOW_GRACE_MS: u64 = 3_000;
pub const SCENE_FOLLOW_ALREADY_ENTERED_MS: u64 = 2_000;
pub const SCENE_FOLLOW_AFTER_DIALOG_CLOSE_MS: u64 = 600;
pub const SCENE_FOLLOW_GIVE_UP_MS: u64 = 15_000;
pub const SCENE_FOLLOW_DUPLICATE_WINDOW_MS: u64 = 20_000;
pub const SCENE_FOLLOW_RECENT_CAPACITY: usize = 16;
pub const SCENE_FOLLOW_NAMES_SENT: usize = 1;
pub const EVENT_CUT_LOADED_BYTE_RVA: u32 = 0x38E_2E01;
pub const EVENT_CUT_EVENT_NAME_RVA: u32 = 0x38E_2E10;
pub const EVENT_CUT_AUDIO_NAME_RVA: u32 = 0x38E_2E30;
pub const EVENT_CUT_NAME_BUFFER_BYTES: usize = 0x20;
pub const EVENT_CUT_LOAD_FUNCTION: &str = "LIB_EventLoad";
pub const EVENT_CUT_LOAD_NO_AUDIO_NUMBER: f64 = -1.0;
pub const UNPLAYED_END_AFTER_START_MAX_MS: u64 = 2_000;

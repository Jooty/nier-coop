mod call_tracker;
mod end_watch;
mod scene_watch;
mod write_plan;

use core::ops::RangeInclusive;

use replicant_proto::StoryArray;

pub use call_tracker::{RunningCall, ScriptCallTracker, talk_tag_function_name};
pub use end_watch::EndWatch;
pub use scene_watch::{SceneWriteWatch, is_scene_write_for_the_host, watches_scene_writes};
pub use write_plan::{
    HostChangeLog, HostWriteOutcome, host_write_outcome, rebaseline_over_mirror_fixes,
    talker_write_batches,
};

pub const END_WATCH_SETTLE_MS: u64 = 2_500;
pub const STORY_WRITE_HOLD_MS: u64 = 3_000;
pub const LOCAL_SCENE_WRITE_WATCH_MAX_MS: u64 = 60_000;
pub const LOCAL_SCENE_CALL_START_WAIT_MAX_MS: u64 = 1_000;
pub const AREA_SCRATCH_GAME_FLAG_IDS: RangeInclusive<u16> = 1401..=1454;
pub const FLAGS_A_LOCAL_SCENE_ALSO_CLEARS: [(StoryArray, u16); 3] = [
    (StoryArray::GameFlags, 1951),
    (StoryArray::SnowFlags, 5),
    (StoryArray::SnowFlags, 1075),
];
pub const HOST_OWN_CHANGE_PRIORITY_MS: u64 = 3_000;
pub const TALK_CALLBACK_ATTRIBUTION_MAX_MS: u64 = 60_000;
pub const TALK_QUEUED_NAME_LIFETIME_MS: u64 = 30_000;
pub const MAX_TALK_QUEUED_NAMES: usize = 32;
pub const TALK_TAG_FUNCTION_NAME_MAX_BYTES: usize = 64;
pub const CALL_FUNCTION_RVA: u32 = 0x3F_04E0;
pub const TALK_TAG_RUN_CALLBACK_RVA: u32 = 0xD_8290;
pub const TALK_LIST_PICK_STATE_RVA: u32 = 0x4E_1300;
pub const MENU_LIST_PICK_STATE_RVA: u32 = 0x4E_1640;
pub const SELECTOR_CONFIRMED_INDEX_RVA: u32 = 0x47_D800;
pub const TALK_WINDOW_PICK_INDEX_OFFSET: u64 = 0x46_306C;
pub const TALK_WINDOW_PICK_TARGET_OFFSET: u64 = 0x46_3070;

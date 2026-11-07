mod gate;
mod steps;

pub use gate::{StoryJumpRefusal, StoryJumpView, check_story_jump};
pub use steps::{JumpStep, destination, loadout_steps};

pub const SET_TUTORIAL_NATIVE_RVA: u32 = 0x70_2B10;
pub const SET_ITEM_NUM_NATIVE_RVA: u32 = 0x6F_C7B0;
pub const SET_PLAYER_WEAPON_NATIVE_RVA: u32 = 0x70_08E0;
pub const SET_UNIQUE_ACTOR_LEVEL_NATIVE_RVA: u32 = 0x70_2F00;
pub const SET_PLAYER_MP_FULL_MODE_NATIVE_RVA: u32 = 0x6F_FE40;
pub const NATIVE_ARGS_HEADER_BYTES: usize = 0x10;
pub const NATIVE_ARG_STRIDE_BYTES: usize = 0x90;
pub const NATIVE_ARG_FIRST_OFFSET_BYTES: usize = 8;
pub const NATIVE_ARG_INTEGER_TYPE_TAG: u32 = 4;
pub const NATIVE_ARG_VALUE_OFFSET_BYTES: usize = 8;
pub const PLAYER_UNIQUE_ACTOR_ID: i32 = -1;

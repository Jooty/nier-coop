mod native;

pub use native::{
    POSITION_NATIVES, PositionNative, describe_argument, describe_arguments,
    native_argument_offset, native_argument_value_offset,
};

pub const SET_PLAYER_POSITION_TO_POINT_NATIVE_RVA: u32 = 0x70_0270;
pub const SET_UNIQUE_ACTOR_POSITION_NATIVE_RVA: u32 = 0x70_39C0;
pub const UNIQUE_ACTOR_POSITION_MAX_ARGUMENTS: usize = 6;

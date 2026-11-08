use crate::position_log::{
    SET_PLAYER_POSITION_TO_POINT_NATIVE_RVA, SET_UNIQUE_ACTOR_POSITION_NATIVE_RVA,
    UNIQUE_ACTOR_POSITION_MAX_ARGUMENTS,
};
use crate::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG,
    NATIVE_ARG_STRIDE_BYTES, NATIVE_ARG_VALUE_OFFSET_BYTES, NATIVE_ARGS_HEADER_BYTES,
    SET_PLAYER_POSITION_NATIVE_RVA,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PositionNative {
    pub name: &'static str,
    pub rva: u32,
    pub max_arguments: usize,
}

pub const POSITION_NATIVES: [PositionNative; 3] = [
    PositionNative {
        name: "_SetPlayerPosition",
        rva: SET_PLAYER_POSITION_NATIVE_RVA,
        max_arguments: 4,
    },
    PositionNative {
        name: "_SetPlayerPositionToPoint",
        rva: SET_PLAYER_POSITION_TO_POINT_NATIVE_RVA,
        max_arguments: 1,
    },
    PositionNative {
        name: "_SetUniqueActorPosition",
        rva: SET_UNIQUE_ACTOR_POSITION_NATIVE_RVA,
        max_arguments: UNIQUE_ACTOR_POSITION_MAX_ARGUMENTS,
    },
];

pub const fn native_argument_offset(index: usize) -> usize {
    NATIVE_ARGS_HEADER_BYTES + NATIVE_ARG_FIRST_OFFSET_BYTES + index * NATIVE_ARG_STRIDE_BYTES
}

pub const fn native_argument_value_offset(index: usize) -> usize {
    native_argument_offset(index) + NATIVE_ARG_VALUE_OFFSET_BYTES
}

pub fn describe_argument(type_tag: u32, value_bits: u64) -> Option<String> {
    match type_tag {
        NATIVE_ARG_INTEGER_TYPE_TAG => Some((value_bits as u32 as i32).to_string()),
        NATIVE_ARG_REAL_TYPE_TAG => Some(format!("{:.1}", f64::from_bits(value_bits))),
        _ => None,
    }
}

pub fn describe_arguments(arguments: &[(u32, u64)]) -> String {
    let parts: Vec<String> = arguments
        .iter()
        .map(|&(tag, bits)| describe_argument(tag, bits).unwrap_or_else(|| format!("?tag{tag}")))
        .collect();
    parts.join(", ")
}

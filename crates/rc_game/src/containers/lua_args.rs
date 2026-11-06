use crate::containers::{
    LUA_ARGUMENT_BASE_OFFSET, LUA_ARGUMENT_STRIDE, LUA_INTEGER_TAG, LUA_NUMBER_TAG,
};

pub fn lua_argument_offset(index: u64) -> u64 {
    LUA_ARGUMENT_BASE_OFFSET + index * LUA_ARGUMENT_STRIDE
}

pub fn lua_integer_argument(tag: i32, payload: [u8; 8]) -> i32 {
    match tag {
        LUA_INTEGER_TAG => i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]),
        LUA_NUMBER_TAG => f64::from_le_bytes(payload) as i32,
        _ => 0,
    }
}

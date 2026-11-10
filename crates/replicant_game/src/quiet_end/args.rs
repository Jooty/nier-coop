pub const NATIVE_STATE_OFFSET: u64 = 8;
pub const LUA_ARGS_OFFSET: u64 = 0x10;
pub const LUA_ARG_TAG_OFFSET: u64 = 8;
pub const LUA_ARG_PAYLOAD_OFFSET: u64 = 0x10;
pub const LUA_ARG_STRIDE: u64 = 0x90;
const LUA_ARG_TAG_INT: i32 = 4;
const LUA_ARG_TAG_NUMBER: i32 = 1;

pub const fn lua_arg_offset(index: u64) -> u64 {
    LUA_ARGS_OFFSET + index * LUA_ARG_STRIDE
}

pub fn lua_arg_as_int(tag: i32, payload: [u8; 8]) -> i32 {
    match tag {
        LUA_ARG_TAG_INT => i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]),
        LUA_ARG_TAG_NUMBER => f64::from_le_bytes(payload) as i32,
        _ => 0,
    }
}

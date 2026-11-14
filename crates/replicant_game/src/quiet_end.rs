mod args;
mod skip;
mod verdict;

pub use args::{
    LUA_ARG_PAYLOAD_OFFSET, LUA_ARG_STRIDE, LUA_ARG_TAG_OFFSET, LUA_ARGS_OFFSET,
    NATIVE_STATE_OFFSET, lua_arg_as_int, lua_arg_offset,
};
pub use skip::{QUIET_END_SKIPPED_NATIVES, QuietEndNative, skipped_native_name};
pub use verdict::{TalkStartVerdict, talk_start_verdict};

pub const LUA_NATIVE_STILL_RUNNING: i32 = 1;
pub const LUA_NATIVE_DONE: i32 = 0;

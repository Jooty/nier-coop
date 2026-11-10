mod native_args;
mod registry;

pub use native_args::{
    DummyNameRefusal, dummy_callback_name, dummy_native_args_len, native_arg_tag_range,
    native_arg_text_range, native_text_arg, native_text_args_len, native_text_from_parts,
};
pub use registry::{DummyHitCallbacks, dummy_hit_log_line};

pub const PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA: u32 = 0x6E_FC90;
pub const DUMMY_CALLBACK_NAME_ARG_INDEX: usize = 2;
pub const MAX_DUMMY_CALLBACKS: usize = 64;

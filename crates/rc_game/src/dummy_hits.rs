mod arrival;
mod fire;
mod native_args;
mod registry;

pub use arrival::{DummyHitArrival, DummyHitArrivalFacts, DummyHitDrop, dummy_hit_arrival};
pub use fire::{
    DummyHitFire, DummyHitFireFacts, DummyHitOrigin, dummy_hit_fire, dummy_hit_log_line,
};
pub use native_args::{DummyNameRefusal, dummy_callback_name, dummy_native_args_len};
pub use registry::DummyHitCallbacks;

pub const PUT_ACTOR_DAMAGE_DUMMY_NATIVE_RVA: u32 = 0x6E_FC90;
pub const DUMMY_CALLBACK_NAME_ARG_INDEX: usize = 2;
pub const MAX_DUMMY_CALLBACKS: usize = 64;

mod gate;
mod names;

pub use gate::{CallbackGate, GateRole, GateStep, LocalFire, PartnerFire};
pub use names::is_gated_callback;

pub const GATED_CALLBACK_NAMES: [&str; 1] = ["Boss_Battle_Start"];
pub const GATED_EVENT_SUFFIX: &str = "_Start";
pub const CALLBACK_GATE_PAIRING_WINDOW_MS: u64 = 20_000;
pub const CALLBACK_GATE_HOST_GRACE_MS: u64 = 3_000;
pub const CALLBACK_GATE_AFTER_DIALOG_CLOSE_MS: u64 = 600;
pub const CALLBACK_GATE_GUEST_GIVE_UP_MS: u64 = 20_000;

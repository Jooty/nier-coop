mod driver;
mod driver_state;
mod menu_hooks;

pub use driver::{install, run_go_to_partner, run_local_jump, tick};
pub use menu_hooks::teleport_hooks;

const NOT_READ_SENTINEL: i32 = -99;

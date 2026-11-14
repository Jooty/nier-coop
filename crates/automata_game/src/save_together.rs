mod guest_gates;
mod save_lines;
mod save_machine;
mod save_readings;
mod save_step;

pub use guest_gates::GuestGates;
pub use save_lines::{
    guest_asked_slot_line, guest_asks_line, guest_dropped_line, guest_gave_up_line,
    guest_heard_line, guest_save_failed_line, guest_save_not_taken_line, guest_save_timed_out_line,
    guest_saved_line, guest_waits_line, host_not_told_line, host_told_line,
};
pub use save_machine::SaveMachine;
pub use save_readings::SaveReadings;
pub use save_step::SaveStep;

pub const GUEST_SAVE_WAIT_LIMIT_MS: u64 = 60_000;
pub const GUEST_SAVE_WRITE_LIMIT_MS: u64 = 60_000;
pub const GUEST_SAVE_NOT_TAKEN_AFTER_MS: u64 = 5_000;
pub const SAVE_WAITS_LOG_INTERVAL_MS: u64 = 1_000;
pub const MOST_HOST_SAVES_TOLD_PER_TICK: u32 = 4;

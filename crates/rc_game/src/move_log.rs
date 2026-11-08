mod input;
mod line;
mod names;
mod rates_dump;
mod sample;
mod spec_dump;
mod speed_cap;
mod trigger;
mod window;

pub use input::{input_block_getter, read_move_axis, read_move_keys};
pub use line::{MoveLineContext, format_move_line};
pub use names::{action_set_for_seed, state_name_for_rva};
pub use rates_dump::{RatesDump, format_rates_dump, read_rates_dump};
pub use sample::{MoveSample, RateChange, read_move_sample};
pub use spec_dump::{
    SPEC_DUMP_SECOND_DUMP_DELAY_MS, SpecDumpMoment, SpecDumpSchedule, SpecTable, format_spec_dump,
    read_spec_table, role_label,
};
pub use speed_cap::{
    SpeedCapLift, SpeedCapLog, SpeedCapStep, format_speed_cap_log, read_speed_cap, should_lift_speed_cap,
    speed_cap_address, write_lifted_speed_cap,
};
pub use trigger::{MoveLogReason, MoveLogTrigger};
pub use window::{MoveLogDue, MoveLogWindow};

pub const MOVE_LOG_DURATION_MS: u64 = 30_000;
pub const MOVE_LOG_INTERVAL_MS: u64 = 1000;
pub const MOVE_LOG_RATE_ID_COUNT: usize = 15;
pub const MOVE_LOG_RATE_CHANNEL_COUNT: usize = 12;
pub const RATE_TABLE_OFFSET: u64 = 0x6_01E0;
pub const RATE_TABLE_ID_STRIDE: usize = 0x60;
pub const RATE_TABLE_CHANNEL_STRIDE: usize = 8;
pub const GROUND_VELOCITY_OFFSET: u64 = 0x5_FDF4;
pub const GROUND_VELOCITY_Z_OFFSET: u64 = 0x5_FDFC;
pub const MOVE_MODE_OFFSET: u64 = 0x6_B778;
pub const SPEC_TABLE_POINTER_OFFSET: u64 = 0x6_2D70;
pub const SPEC_TABLE_BYTE_COUNT: usize = 0x4A4;
pub const SPEC_TABLE_WORDS_PER_LINE: usize = 8;
pub const SPEED_CAP_RATE_ID: usize = 5;
pub const SPEED_CAP_RATE_CHANNEL: usize = 2;
pub const SPEED_CAP_THRESHOLD: f32 = 0.05;
pub const SPEED_CAP_LIFTED_VALUE: f32 = 1.0;
pub const SPEED_CAP_GIVE_UP_AFTER_RESETS: u32 = 20;
pub const STATE_161F0_OFFSET: u64 = 0x1_61F0;
pub const APPLIED_STEP_OFFSET: u64 = 0x1A0;
pub const DISPLACEMENT_PENDING_FLAG_OFFSET: u64 = 0x1D0;
pub const CURRENT_STATE_POINTER_OFFSET: u64 = 0x1_6200;
pub const STATE_LOCK_OFFSET: u64 = 0x6_B920;
pub const STATE_LEVEL_OFFSET: u64 = 0x6_B908;
pub const STATE_LIMIT_OFFSET: u64 = 0x6_B90C;
pub const MOVE_BLOCK_OFFSET: u64 = 0x6_B784;
pub const DAMAGE_MODE_OFFSET: u64 = 0x6_B96C;
pub const COLLISION_FLAGS_OFFSET: u64 = 0x6_2C00;
pub const LOCK_BYTE_FLAGS_OFFSET: u64 = 0x1_6074;
pub const STEPS_PER_SECOND: f32 = 60.0;
pub const ACTOR_INPUT_BLOCK_GETTER_SLOT_OFFSET: u64 = 0xCD8;
pub const CODE_SECTION_END_RVA: u32 = 0xAC_0000;
pub const INPUT_BLOCK_MOVE_AXIS_X_OFFSET: u64 = 0x4C;
pub const INPUT_BLOCK_MOVE_AXIS_Y_OFFSET: u64 = 0x54;
pub const KEYBOARD_DEVICE_POINTER_RVA: u32 = 0x514_6EC0;
pub const KEY_BIT_TABLE_RVA: u32 = 0xCE_0700;
pub const KEY_SET_OFFSET: u64 = 0x10;
pub const KEY_BIT_COUNT: u64 = 256;
pub const MOVE_KEY_VIRTUAL_KEYS: [u32; 4] = [0x57, 0x41, 0x53, 0x44];

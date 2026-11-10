mod gate;
mod hold_reading;
mod load_signal;
mod loader_code;

pub use gate::{AloneReason, GateInputs, GateState, GateStep, LoadGate, PartnerLoad, Release};
pub use hold_reading::HoldReading;
pub use load_signal::LoadSignal;
pub use loader_code::{LoaderAddresses, loader_code_is_the_noted_one};

pub const PARTNER_READY_WAIT_LIMIT_MS: u64 = 45_000;
pub const RELEASE_LEAD_MS: u64 = 500;
pub const HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS: u64 = 1_500;
pub const LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS: u64 = 3_000;
pub const PARTNER_LOAD_SIGNAL_STALE_MS: u64 = 1_500;
pub const LOAD_END_HOLD_HARD_LIMIT_MS: u64 = 60_000;
pub const STATE_WITHOUT_BODY_SEND_INTERVAL_MS: u64 = 50;

pub const SCENE_LOADER_LAST_STAGE: i32 = 4;
pub const LOAD_DONE_REQUESTED: i32 = 1;
pub const LOAD_DONE_NOT_REQUESTED: i32 = 0;
pub const SCENE_LOADER_FUNCTION_START: [u8; 34] = [
    0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x56, 0x48, 0x8D, 0x6C, 0x24, 0xD0, 0x48, 0x81, 0xEC, 0x30,
    0x01, 0x00, 0x00, 0x48, 0xC7, 0x45, 0x28, 0xFE, 0xFF, 0xFF, 0xFF, 0x45, 0x33, 0xF6, 0x44, 0x89,
    0x75, 0x60,
];
pub const SCENE_LOADER_STAGE_READ_OFFSET: usize = 0x22;
pub const SCENE_LOADER_STAGE_READ_OPCODE: [u8; 2] = [0x8B, 0x0D];
pub const SCENE_LOADER_ANSWER_COMPARE_OFFSET: usize = 0x54;
pub const SCENE_LOADER_ANSWER_COMPARE_OPCODE: [u8; 3] = [0x44, 0x39, 0x35];
pub const SCENE_LOADER_REQUEST_WRITE_OFFSET: usize = 0xC4E;
pub const SCENE_LOADER_STAGE_WRITE_OFFSET: usize = 0xC58;
pub const STORE_OF_A_FOUR_BYTE_VALUE_OPCODE: [u8; 2] = [0xC7, 0x05];
pub const SCENE_LOADER_CODE_CHECKED_LEN: usize = 0xC62;

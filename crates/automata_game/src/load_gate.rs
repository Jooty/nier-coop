mod boot_screen;
mod gate;
mod hold_reading;
mod load_signal;
mod loader_code;
mod loading_screen;
mod still_loading_code;
mod system_data_band;

pub use boot_screen::{
    BootLoadEndAction, BootLoadPass, BootScreenInputs, BootScreenKeep, BootScreenState,
    LoaderReading, boot_load_end_action,
};
pub use gate::{AloneReason, GateInputs, GateState, GateStep, LoadGate, PartnerLoad, Release};
pub use hold_reading::HoldReading;
pub use load_signal::{LoadSignal, LocalLoadSignal};
pub use loader_code::{LoaderAddresses, loader_code_is_the_noted_one};
pub use loading_screen::LoadingScreenReading;
pub use still_loading_code::still_loading_test_code_is_the_noted_one;
pub use system_data_band::{
    BootScreenDialogs, SystemDialogAnswer, system_dialog_open_code_is_the_noted_one,
    title_system_data_step_code_is_the_noted_one,
};

pub const PARTNER_READY_WAIT_LIMIT_MS: u64 = 45_000;
pub const RELEASE_LEAD_MS: u64 = 500;
pub const HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS: u64 = 1_500;
pub const LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS: u64 = 3_000;
pub const RELEASE_TIME_THIS_LONG_PAST_MEANS_THE_PARTNER_LEFT_ALONE_MS: u64 = 2_000;
pub const PARTNER_LOAD_SIGNAL_STALE_MS: u64 = 1_500;
pub const LOAD_END_HOLD_HARD_LIMIT_MS: u64 = 60_000;
pub const STATE_WITHOUT_BODY_SEND_INTERVAL_MS: u64 = 50;

pub const SCENE_LOADER_LAST_STAGE: i32 = 4;
pub const LOAD_DONE_REQUESTED: i32 = 1;
pub const LOAD_DONE_NOT_REQUESTED: i32 = 0;
pub const LOADING_SCREEN_ANSWERED: i32 = 1;
pub const LOADING_SCREEN_NOT_ANSWERED: i32 = 0;
pub const SCENE_STATE_WHILE_A_LOAD_RUNS: i32 = 0;
pub const BOOT_SCREEN_KEPT_PAST_THE_BOOT_LOAD_AT_MOST_MS: u64 = 45_000;
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

pub const STILL_LOADING_TEST_FUNCTION_START: [u8; 10] =
    [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9, 0xE8];
pub const STILL_LOADING_TEST_SCENE_STATE_COMPARE_OFFSET: usize = 0x1A;
pub const STILL_LOADING_TEST_SCENE_STATE_COMPARE_OPCODE: [u8; 2] = [0x83, 0x3D];
pub const STILL_LOADING_TEST_FUNCTION_END_OFFSET: usize = 0x62;
pub const STILL_LOADING_TEST_FUNCTION_END: [u8; 21] = [
    0x74, 0x0B, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x48, 0x83, 0xC4, 0x20, 0x5B, 0xC3, 0x33, 0xC0, 0x48,
    0x83, 0xC4, 0x20, 0x5B, 0xC3,
];
pub const STILL_LOADING_TEST_CODE_CHECKED_LEN: usize = 0x77;

pub const TITLE_SYSTEM_DATA_STEP_FUNCTION_START: [u8; 34] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48,
    0x63, 0x41, 0x04, 0x33, 0xDB, 0x48, 0x8B, 0xF9, 0xBE, 0xFF, 0xFF, 0xFF, 0xFF, 0x83, 0xF8, 0x06,
    0x77, 0x53,
];
pub const TITLE_SYSTEM_DATA_STEP_DIALOG_OPEN_CALL_OFFSET: usize = 0x26B;
pub const NEAR_CALL_OPCODE: [u8; 1] = [0xE8];
pub const TITLE_SYSTEM_DATA_STEP_STORES_THE_DIALOG_HANDLE_AND_TESTS_FOR_NONE: [u8; 9] =
    [0x89, 0x87, 0x90, 0x09, 0x00, 0x00, 0x83, 0xF8, 0xFF];
pub const TITLE_SYSTEM_DATA_STEP_CODE_CHECKED_LEN: usize = 0x279;
pub const SYSTEM_DIALOG_OPEN_FUNCTION_START: [u8; 0x56] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40, 0x44, 0x8B, 0x54, 0x24, 0x70, 0x49,
    0x8B, 0xD8, 0x8B, 0xF9, 0x41, 0xBB, 0x03, 0x00, 0x00, 0x00, 0x41, 0x83, 0xEA, 0x01, 0x74, 0x10,
    0x41, 0x83, 0xFA, 0x02, 0x74, 0x05, 0x45, 0x33, 0xC0, 0xEB, 0x0B, 0x45, 0x8B, 0xC3, 0xEB, 0x06,
    0x41, 0xB8, 0x01, 0x00, 0x00, 0x00, 0x41, 0x8B, 0xC8, 0x83, 0xE9, 0x01, 0x74, 0x10, 0x83, 0xF9,
    0x02, 0x74, 0x08, 0x41, 0xBB, 0x02, 0x00, 0x00, 0x00, 0xEB, 0x03, 0x45, 0x33, 0xDB, 0xC7, 0x44,
    0x24, 0x38, 0x00, 0x00, 0x00, 0x00,
];
pub const SYSTEM_DIALOG_NOT_OPENED_HANDLE: i32 = -1;
pub const SYSTEM_DATA_STAGE_WHILE_LOADING: i32 = 4;
pub const SYSTEM_DIALOG_LINES_WHILE_THE_BOOT_SCREEN_IS_KEPT_AT_MOST: u32 = 8;

pub const LOADING_SCREEN_STATE_OFFSET: u32 = 0x178;
pub const LOADING_SCREEN_SHOWN_STATE: i32 = 3;
pub const LOADING_SCREEN_OPENING_PART_OFFSET: u32 = 0x1E8;
pub const LOADING_SCREEN_STEP_OFFSET: u32 = 0x1E0;
pub const LOADING_SCREEN_MODE_OFFSET: u32 = 0x1AC;
pub const LOADING_SCREEN_SHOWN_PART_OFFSET: u32 = 0x1C0;
pub const UI_PART_FLAGS_OFFSET: u32 = 0x58;
pub const UI_PART_ASKED_TO_PLAY_OUT_BIT: u32 = 0x10_0000;
pub const UI_PART_HAS_PLAYED_OUT_BIT: u32 = 0x2000_0000;
pub const LOADING_SCREEN_PLAY_OUT_WAIT_LIMIT_MS: u64 = 20_000;
pub const LOADING_SCREEN_LINES_LOGGED_AT_MOST: u32 = 16;

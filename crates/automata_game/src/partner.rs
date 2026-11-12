mod body_fields;
mod body_snapshot;
mod camera_keep;
mod controller_code;
mod drive_setting;
mod engine_partner_choice;
mod error_bands;
mod free_rule;
mod hello_rule;
mod input_mode_keep;
mod partner_body_status;
mod partner_data_wait;
mod partner_machine;
mod partner_presence;
mod pod_aim;
mod press_timers;
mod remote_input;
mod send_pacer;
mod state_rule;
mod wanted_character;

use crate::character::Character;

pub use body_fields::{BodyFields, GiveBack, gives_fields_back};
pub use body_snapshot::{BodySnapshot, speed_for_wire, state_id_for_wire};
pub use camera_keep::CameraKept;
pub use controller_code::{code_starts_with, controller_calls_match, process_buttons_stores_match};
pub use drive_setting::PartnerDrive;
pub use engine_partner_choice::{
    BuddyFlagWrite, BuddyFlags, EngineChoiceLeftAlone, EnginePartnerChoice, EnginePartnerReadings,
    buddy_flag_hash, choose_engine_partner, engine_partner_spawn_code_is_the_noted_one,
};
pub use error_bands::{PositionCorrection, PositionCorrector, facing_error, facing_needs_write};
pub use free_rule::{
    FREE_MUST_HOLD_BEFORE_IT_COUNTS_MS, FreeReadings, GlobalGates, HeldFree, HeroGates, NotFree,
    ReadyLatch, not_free_reason,
};
pub use hello_rule::{HelloRule, HeroIdentity};
pub use input_mode_keep::{InputModeKept, body_update_copies_the_input_mode};
pub use partner_body_status::partner_body_status_line;
pub use partner_data_wait::{PartnerDataReading, PartnerDataStep, PartnerDataWait};
pub use partner_machine::{
    BuddyReading, HeroReading, LOCAL_NOT_FREE_HOLD_BEFORE_RELEASE_MS, PartnerAction, PartnerMachine, PartnerReadings, PartnerStep,
};
pub use partner_presence::{partner_is_in_play, partner_positions_apply};
pub use pod_aim::{AimRayPoints, PodAim};
pub use press_timers::button_words_after_presses;
pub use remote_input::{InputFeed, RemoteInput};
pub use send_pacer::SendPacer;
pub use state_rule::{
    RecordedStart, RemoteState, RemoteStateIds, StateDecision, StateRule, body_state_is_plain,
    is_never_started, is_started_on_the_body,
};
pub use wanted_character::{partner_character_note, wanted_partner};

pub const DEFAULT_PARTNER_DRIVE: PartnerDrive = PartnerDrive::Input;

pub const ENTITY_HANDLE_OFFSET: u32 = 0x30;
pub const BODY_POSITION_OFFSET: u32 = 0x50;
pub const BODY_ROTATION_EULER_OFFSET: u32 = 0x90;
pub const BODY_FACING_OFFSET: u32 = 0x94;
pub const BODY_FLAGS_OFFSET: u32 = 0x598;
pub const BODY_MODEL_ID_OFFSET: u32 = 0x5B8;
pub const BODY_SUSPEND_FLAGS_OFFSET: u32 = 0x5C0;
pub const BODY_ENTITY_POINTER_OFFSET: u32 = 0x610;
pub const BODY_STATE_OFFSET: u32 = 0x670;
pub const BODY_INPUT_BLANK_SWITCH_OFFSET: u32 = 0x680;
pub const BODY_BASE_HEALTH_OFFSET: u32 = 0x858;
pub const BODY_STATE_VARIANT_OFFSET: u32 = 0x674;
pub const BODY_STATE_B_OFFSET: u32 = 0x678;
pub const BODY_STATE_C_OFFSET: u32 = 0x67C;
pub const BODY_CONTROLLER_OFFSET: u32 = 0xCA0;
pub const BODY_PARTNER_MARK_WORD_OFFSET: u32 = 0x1730;
pub const BODY_HELD_BUTTONS_OFFSET: u32 = 0x13D8;
pub const BODY_PRESSED_BUTTONS_OFFSET: u32 = 0x13DC;
pub const BODY_SPEED_OFFSET: u32 = 0x1434;
pub const BODY_WORLD_FACING_OFFSET: u32 = 0x143C;
pub const BODY_HEALTH_OFFSET: u32 = 0x1_0668;
pub const BODY_PLAYER_CONTROLLED_OFFSET: u32 = 0x1_0B64;
pub const BODY_BERSERK_SWITCH_OFFSET: u32 = 0x1_2AC4;
pub const BODY_PARTNER_SLOT_HANDLE_OFFSET: u32 = 0x1_647C;
pub const BODY_RUN_SPEED_TYPE_OFFSET: u32 = 0x1_6488;
pub const BODY_DISMISSED_FLAG_OFFSET: u32 = 0x1_64EC;
pub const BODY_INPUT_IGNORE_TIMER_OFFSET: u32 = 0x1_6A08;
pub const BODY_UPDATE_GATE_OFFSET: u32 = 0x1_6D20;
pub const BODY_BREAK_FIELDS_OFFSET: u32 = 0x1_6ED4;
pub const BODY_BREAK_FIELD_COUNT: usize = 4;
pub const BODY_INPUT_BLANK_TIMER_OFFSET: u32 = 0x1_7914;
pub const CONTROLLER_PAD_ENABLE_OFFSET: u32 = 0x720;
pub const CONTROLLER_MOUSE_ENABLE_OFFSET: u32 = 0x724;
pub const CONTROLLER_KEYBOARD_ENABLE_OFFSET: u32 = 0x728;
pub const CONTROLLER_HELD_BUTTONS_OFFSET: u32 = 0x738;
pub const CONTROLLER_BUTTON_WORDS_OFFSET: u32 = 0x770;
pub const CONTROLLER_PRESS_TIMER_START_OFFSET: u32 = 0x790;
pub const CONTROLLER_SPEED_OFFSET: u32 = 0x794;
pub const CONTROLLER_WORLD_FACING_OFFSET: u32 = 0x79C;
pub const STATE_START_VTABLE_OFFSET: u32 = 0x90;
pub const CAMERA_LOCK_TARGET_HANDLES_OFFSET: u32 = 0x37C;
pub const CAMERA_LOCK_TARGET_HANDLE_COUNT: usize = 2;
pub const CAMERA_MODE_OFFSET: u32 = 0x390;
pub const CAMERA_WORD_ZEROED_BY_A_MODE_REQUEST_OFFSET: u32 = 0x408;
pub const CAMERA_FLAG_BIT_CLEARED_BY_A_MODE_REQUEST: u32 = 0x8000;
pub const CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS: [u32; 3] = [0x524, 0x588, 0x58C];

pub const CONTROLLER_CALLS_LEN: usize = 26;
pub const PROCESS_BUTTONS_FIRST_STORE_AT: usize = 0x11;
pub const PROCESS_BUTTONS_FIRST_STORE: [u8; 7] = [0x48, 0x89, 0x81, 0x38, 0x07, 0x00, 0x00];
pub const PROCESS_BUTTONS_SECOND_STORE_AT: usize = 0x1A;
pub const PROCESS_BUTTONS_SECOND_STORE: [u8; 7] = [0x48, 0x89, 0x81, 0x40, 0x07, 0x00, 0x00];
pub const PROCESS_BUTTONS_CHECKED_LEN: usize = 0x21;
pub const SET_BUDDY_AI_TYPE_FUNCTION_START: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20];
pub const PARTNER_WALK_IN_STATE_3B_FUNCTION_START: [u8; 8] =
    [0x40, 0x55, 0x53, 0x48, 0x8D, 0x6C, 0x24, 0xF8];
pub const START_ANIMATION_FUNCTION_START: [u8; 26] = [
    0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18, 0x57,
    0x48, 0x83, 0xEC, 0x20, 0x8B, 0x81, 0x70, 0x06, 0x00, 0x00,
];
pub const ACTION_UPDATE_FUNCTION_START: [u8; 22] = [
    0x40, 0x55, 0x53, 0x56, 0x57, 0x41, 0x56, 0x48, 0x8D, 0xAC, 0x24, 0x30, 0xFD, 0xFF, 0xFF, 0x48,
    0x81, 0xEC, 0xD0, 0x03, 0x00, 0x00,
];
pub const BODY_UPDATE_FUNCTION_START: [u8; 20] = [
    0x40, 0x55, 0x57, 0x41, 0x57, 0x48, 0x8D, 0xAC, 0x24, 0x60, 0xFE, 0xFF, 0xFF, 0x48, 0x81, 0xEC,
    0xA0, 0x02, 0x00, 0x00,
];
pub const BODY_UPDATE_INPUT_MODE_COPY_LEN: usize = 18;
pub const PAD_INPUT_MODE: u32 = 0;

pub const POD_SHOT_UPDATE_FUNCTION_START: [u8; 38] = [
    0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x10, 0x48, 0x89, 0x70, 0x18, 0x48, 0x89, 0x78, 0x20, 0x55,
    0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0xA8, 0xC8, 0xFA, 0xFF, 0xFF, 0x48,
    0x81, 0xEC, 0x10, 0x06, 0x00, 0x00,
];
pub const CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START: [u8; 36] = [
    0x48, 0x83, 0xEC, 0x38, 0x48, 0x8B, 0x41, 0x60, 0x4D, 0x8B, 0xD0, 0x4C, 0x8B, 0xDA, 0x48, 0x89,
    0x44, 0x24, 0x20, 0x49, 0x8B, 0xD2, 0x49, 0x8B, 0xCB, 0x4C, 0x8D, 0x48, 0x40, 0x4C, 0x8D, 0x80,
    0x58, 0x02, 0x00, 0x00,
];
pub const SCREEN_DEPTH_OF_THE_NEAR_AIM_POINT: f32 = 0.0;
pub const SCREEN_DEPTH_OF_THE_FAR_AIM_POINT: f32 = 1.0;
pub const LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT: f32 = 1_000.0;
pub const SHORTEST_AIM_RAY_THAT_GIVES_A_DIRECTION: f32 = 0.001;
pub const POD_AIM_DIRECTION_LENGTH_ERROR_ALLOWED: f32 = 0.01;
pub const HIGHEST_POD_AIM_EYE_COORDINATE: f32 = 1_000_000.0;
pub const HERO_POD_AIM_IS_SENT_FOR_MS: u64 = 250;

pub const TASK_SCHEDULER_CURRENT_TASK_OFFSET: u32 = 0x20;
pub const SPAWN_OBJECT_READ_VARIANT: u32 = 0;
pub const WAIT_FOR_OBJECT_READ_FUNCTION_START: [u8; 7] = [0xBA, 0x01, 0x00, 0x00, 0x00, 0xC7, 0x05];

pub const BUDDY_9S_FLAG_HASH: u32 = 0x96DD_1A8B;
pub const BUDDY_2B_FLAG_HASH: u32 = 0x1F99_E3B2;
pub const BUDDY_A2_FLAG_HASH: u32 = 0x4B8E_B8BB;
pub const BUDDY_AUTO_FLAG_HASH: u32 = 0x60D9_4EC2;
pub const ORDER_THE_ENGINE_TESTS_THE_BUDDY_FLAGS_IN: [Character; 3] =
    [Character::NineS, Character::TwoB, Character::A2];
pub const ENGINE_PARTNER_SPAWN_FUNCTION_START: [u8; 8] =
    [0x40, 0x55, 0x53, 0x56, 0x48, 0x8D, 0x6C, 0x24];
pub const ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN: usize = 0x70;
pub const LOAD_OF_A_FOUR_BYTE_VALUE_INTO_THE_SECOND_ARGUMENT_OPCODE: u8 = 0xBA;
pub const PARTNER_DATA_WAIT_LIMIT_MS: u64 = 10_000;

pub const SUSPENDED_FLAG_BIT: u32 = 0x4;
pub const SCENE_INPUT_OFF_BIT: u32 = 0x20_0000;
pub const EVENT_INPUT_OFF_BIT: u32 = 0x2000_0000;
pub const PARTNER_MARK_BIT: u32 = 0x1_0000;
pub const COMPUTER_CONTROLLED_FLAG_BITS: u32 = 0x41;
pub const FLAG_BITS_CLEARED_ON_TAKEOVER: u32 = 0xD;
pub const PLAYER_RUN_SPEED_TYPE: i32 = 0;
pub const FOLLOWING_PARTNER_RUN_SPEED_TYPE: i32 = 1;
pub const DISMISSED_BY_THE_ENGINE: i32 = 1;
pub const BUTTON_WORD_COUNT: usize = 8;
pub const HIGHEST_STATE_ID: u16 = 0xD3;
pub const STICK_LENGTH_ABOVE_WHICH_THE_GAME_WALKS: f32 = 300.0;
pub const HIGHEST_SPEED_SENT: f32 = 1000.0;
pub const SPAWNED_BODY_NAME: &str = "partner";
pub const SPAWN_OFFSET_FROM_HERO: [f32; 3] = [1.0, 0.0, 0.0];

pub const SNAPSHOT_SEND_INTERVAL_MS: u64 = 33;
pub const REMOTE_STATE_STALE_MS: u64 = 250;
pub const PARTNER_SILENCE_BEFORE_GIVE_BACK_MS: u64 = 10_000;
pub const ENGINE_PARTNER_WAIT_MS: u64 = 5_000;
pub const SPAWN_RETRY_INTERVAL_MS: u64 = 5_000;
pub const DESTROY_RETRY_INTERVAL_MS: u64 = 1_000;
pub const POSITION_ERROR_LEFT_ALONE: f32 = 0.1;
pub const POSITION_ERROR_TELEPORTED: f32 = 1.0;
pub const POSITION_NUDGE_FRACTION: f32 = 0.2;
pub const TELEPORT_MIN_INTERVAL_MS: u64 = 500;
pub const FACING_ERROR_WRITTEN_RADIANS: f32 = 1.0;
pub const DIFFERING_MESSAGES_BEFORE_STATE_START: u32 = 6;
pub const NEVER_STARTED_STATES: [u16; 7] = [0x1B, 0x35, 0x36, 0x3B, 0x41, 0x68, 0x6A];
pub const STATES_STARTED_ON_THE_BODY: [u16; 4] = [0, 1, 2, 0xBF];

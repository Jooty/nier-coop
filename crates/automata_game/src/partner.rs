mod body_fields;
mod body_snapshot;
mod drive_setting;
mod error_bands;
mod hello_rule;
mod partner_machine;
mod remote_input;
mod send_pacer;
mod state_rule;
mod wanted_character;

pub use body_fields::BodyFields;
pub use body_snapshot::{BodySnapshot, button_mask_of, button_words_of, state_id_for_wire};
pub use drive_setting::PartnerDrive;
pub use error_bands::{PositionCorrection, PositionCorrector, facing_error, facing_needs_write};
pub use hello_rule::HelloRule;
pub use partner_machine::{
    BuddyReading, HeroReading, PartnerAction, PartnerMachine, PartnerReadings, PartnerStep,
};
pub use remote_input::{InputFeed, RemoteInput};
pub use send_pacer::SendPacer;
pub use state_rule::{RemoteStateIds, StateDecision, StateRule, is_never_started};
pub use wanted_character::{partner_character_note, wanted_partner};

pub const DEFAULT_PARTNER_DRIVE: PartnerDrive = PartnerDrive::State;

pub const ENTITY_HANDLE_OFFSET: u32 = 0x30;
pub const BODY_POSITION_OFFSET: u32 = 0x50;
pub const BODY_ROTATION_EULER_OFFSET: u32 = 0x90;
pub const BODY_FACING_OFFSET: u32 = 0x94;
pub const BODY_FLAGS_OFFSET: u32 = 0x598;
pub const BODY_MODEL_ID_OFFSET: u32 = 0x5B8;
pub const BODY_SUSPEND_FLAGS_OFFSET: u32 = 0x5C0;
pub const BODY_ENTITY_POINTER_OFFSET: u32 = 0x610;
pub const BODY_STATE_OFFSET: u32 = 0x670;
pub const BODY_STATE_VARIANT_OFFSET: u32 = 0x674;
pub const BODY_STATE_B_OFFSET: u32 = 0x678;
pub const BODY_STATE_C_OFFSET: u32 = 0x67C;
pub const BODY_CONTROLLER_OFFSET: u32 = 0xCA0;
pub const BODY_PARTNER_MARK_WORD_OFFSET: u32 = 0x1730;
pub const BODY_HELD_BUTTONS_OFFSET: u32 = 0x13D8;
pub const BODY_PRESSED_BUTTONS_OFFSET: u32 = 0x13DC;
pub const BODY_BUTTON_WORDS_OFFSET: u32 = 0x1410;
pub const BODY_SPEED_OFFSET: u32 = 0x1434;
pub const BODY_WORLD_FACING_OFFSET: u32 = 0x143C;
pub const BODY_HEALTH_OFFSET: u32 = 0x1_0668;
pub const BODY_PLAYER_CONTROLLED_OFFSET: u32 = 0x1_0B64;
pub const BODY_HEALTH_FLOOR_SWITCH_OFFSET: u32 = 0x1_2AC4;
pub const BODY_RUN_SPEED_TYPE_OFFSET: u32 = 0x1_6488;
pub const BODY_DISMISSED_FLAG_OFFSET: u32 = 0x1_64EC;
pub const BODY_INPUT_IGNORE_TIMER_OFFSET: u32 = 0x1_6A08;
pub const BODY_UPDATE_GATE_OFFSET: u32 = 0x1_6D20;
pub const CONTROLLER_PAD_ENABLE_OFFSET: u32 = 0x720;
pub const CONTROLLER_SCRIPT_ENABLE_OFFSET: u32 = 0x724;
pub const CONTROLLER_COMPUTER_ENABLE_OFFSET: u32 = 0x728;
pub const STATE_START_VTABLE_OFFSET: u32 = 0x90;

pub const SUSPENDED_FLAG_BIT: u32 = 0x4;
pub const PARTNER_MARK_BIT: u32 = 0x1_0000;
pub const COMPUTER_CONTROLLED_FLAG_BITS: u32 = 0x41;
pub const FLAG_BITS_CLEARED_ON_TAKEOVER: u32 = 0xD;
pub const PLAYER_RUN_SPEED_TYPE: i32 = 0;
pub const BUTTON_WORD_COUNT: usize = 8;
pub const PRESSED_BUTTON_WORD_VALUE: u32 = 0x14;
pub const SPAWNED_BODY_NAME: &str = "partner";
pub const SPAWN_OFFSET_FROM_HERO: [f32; 3] = [1.0, 0.0, 0.0];

pub const SNAPSHOT_SEND_INTERVAL_MS: u64 = 33;
pub const REMOTE_STATE_STALE_MS: u64 = 1_000;
pub const ENGINE_PARTNER_WAIT_MS: u64 = 5_000;
pub const SPAWN_RETRY_INTERVAL_MS: u64 = 5_000;
pub const DESTROY_RETRY_INTERVAL_MS: u64 = 1_000;
pub const POSITION_ERROR_LEFT_ALONE: f32 = 0.1;
pub const POSITION_ERROR_TELEPORTED: f32 = 1.0;
pub const POSITION_NUDGE_FRACTION: f32 = 0.2;
pub const TELEPORT_MIN_INTERVAL_MS: u64 = 500;
pub const FACING_ERROR_WRITTEN_RADIANS: f32 = 1.0;
pub const DIFFERING_MESSAGES_BEFORE_STATE_START: u32 = 6;
pub const NEVER_STARTED_STATES: [u16; 6] = [0x1B, 0x35, 0x36, 0x41, 0x68, 0x6A];

mod build_gate;
mod codec;
mod combat_event;
mod container_opened;
mod cutscene_skip;
mod dialogue_line;
mod door_sync;
mod ending_records;
mod enemy;
mod enemy_combat;
mod error;
mod fade_sync;
mod follow_phase;
mod game_readiness;
mod item_pickup;
mod lobby;
mod message;
mod motion_state;
mod music_state;
mod npc;
mod partner_activity;
mod partner_body;
mod partner_effect;
mod partner_notice;
mod player_settings;
mod player_state;
mod script_message;
mod script_roll;
mod session_options;
mod sky_state;
mod story_call;
mod story_progress;
mod talk_choice;
mod tutorial_prompt;
mod weapon_state;

pub use build_gate::BuildGate;
pub use codec::{decode, encode};
pub use combat_event::CombatEvent;
pub use container_opened::{ContainerFlag, ContainerItem, ContainerOpened};
pub use cutscene_skip::CutsceneSkipAsk;
pub use dialogue_line::{DialogueLine, DialogueUpdate};
pub use door_sync::{DoorAction, DoorChange, DoorReport, DoorState};
pub use ending_records::{ENDING_ACCOMPLISHED_BYTES, ENDING_COSTUME_BYTES, EndingRecords};
pub use enemy::{EnemyGone, EnemyGoneReason, EnemyId, EnemySnapshot, EnemySpawned, EnemyState};
pub use enemy_combat::{EnemyDrop, EnemyHit, EnemyKilled, EnemyWordDrop, PlayerHit};
pub use error::WireError;
pub use fade_sync::FadeSync;
pub use follow_phase::FollowPhase;
pub use game_readiness::GameReadiness;
pub use item_pickup::{ItemPickup, PickupSource};
pub use lobby::{LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyReady, LobbyStart, MAX_LOBBY_SLOT};
pub use message::{Hello, Message, PathInfo, Ping, Pong, Reject, RejectReason, Welcome};
pub use motion_state::MotionState;
pub use music_state::{
    MAX_MUSIC_FADE_SECONDS, MAX_MUSIC_POSITION_MS, MAX_MUSIC_VOLUME_ABS, MUSIC_LAYER_COUNT,
    MusicState, NO_MUSIC_TRACK,
};
pub use npc::{NpcId, NpcSnapshot, NpcState, NpcTalk};
pub use partner_activity::PartnerActivity;
pub use partner_body::{
    BodyPackName, MAX_BODY_PACK_NAME_BYTES, NO_MOTION_NAME_HASH, PartnerBody, motion_name_hash,
};
pub use partner_effect::{
    BulletEffectPlay, EFFECT_ATTACH_BYTES, EFFECT_PARAM_BYTES, EffectPlay, EffectStop,
    MAX_EFFECT_BONE_BYTES, MAX_EFFECT_NAME_BYTES, MAX_SOUND_NAME_BYTES, SoundPlay,
};
pub use partner_notice::PartnerNotice;
pub use player_settings::PlayerSettings;
pub use player_state::PlayerState;
pub use script_message::{MAX_SCRIPT_MESSAGE_NUMBER, ScriptMessage};
pub use script_roll::{MAX_SCRIPT_ROLL_ORDINAL, ScriptRoll, ScriptRollValue};
pub use session_options::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, MAX_ENEMY_DAMAGE_PERCENT,
    MAX_ENEMY_HEALTH_PERCENT, SessionOptions,
};
pub use sky_state::{SkyAssets, SkyRecordChunk, SkyState};
pub use story_call::{
    MAX_STORY_CALL_ARGS, MAX_STORY_CALL_NAME_BYTES, MAX_STORY_CALL_TEXT_BYTES, StoryCall,
    StoryCallArg, StoryRefusalReason, StoryRefused,
};
pub use story_progress::{
    StoryArray, StoryArrayPart, StoryChange, StoryChanges, StoryDigest, StoryRequest, StoryWrite,
};
pub use talk_choice::{TalkChoice, TalkChoiceKind};
pub use tutorial_prompt::{FIRST_TUTORIAL_ID, LAST_TUTORIAL_ID, TutorialPrompt};
pub use weapon_state::WeaponState;

pub const PROTOCOL_VERSION: u16 = 10;
pub const MAX_FADE_SECONDS: f32 = 30.0;
pub const MAX_MESSAGE_BYTES: usize = 512;
pub const MAX_DISPLAY_NAME_BYTES: usize = 32;
pub const MAX_PLAYERS: u8 = 2;
pub const MAX_PHASE_POINT: i32 = 99;
pub const MAX_HP: u32 = 1_000_000;
pub const MAX_POSITION_ABS: f32 = 1_000_000.0;
pub const MAX_YAW_ABS_RADIANS: f32 = 16.0;
pub const MAX_MOTION_FRAME: f32 = 100_000.0;
pub const MAX_MOTION_SPEED_ABS: f32 = 32.0;
pub const MAX_WEAPON_ID: u8 = 63;
pub const MAX_WEAPON_LEVEL: u8 = 3;
pub const ENEMY_GROUP_COUNT: u16 = 0x400;
pub const MAX_ENEMIES_PER_SNAPSHOT: usize = 5;
pub const MAX_ENEMY_SNAPSHOT_PARTS: u8 = 6;
pub const NPC_UNGROUPED: u16 = u16::MAX;
pub const MAX_NPCS_PER_SNAPSHOT: usize = 10;
pub const MAX_NPC_SNAPSHOT_PARTS: u8 = 8;
pub const MAX_SKY_AREA: i32 = 127;
pub const MAX_SKY_BLEND_SECONDS: f32 = 120.0;
pub const MAX_ITEM_ID: u16 = 0x2FF;
pub const MAX_ITEM_COUNT: u8 = 99;
pub const MAX_WORD_ID: u16 = 0x7F;
pub const MAX_TALK_ID: u32 = 0x3_FFFF;
pub const MAX_DIALOGUE_LINE_INDEX: u16 = 0x0FFF;
pub const MAX_SKY_RECORD_BYTES: usize = 0xAA0;
pub const MAX_SKY_RECORD_CHUNK_BYTES: usize = 384;
pub const SKY_ASSET_SLOTS: usize = 5;
pub const MAX_HIT_DAMAGE: u32 = 100_000;
pub const MAX_SKY_ASSET_NAME_BYTES: usize = 80;
pub const MAX_DOOR_LAYOUT_ID: u16 = 9999;
pub const MAX_DOORS_PER_REPORT: usize = 60;
pub const STORY_ARRAY_BYTES: usize = 0x100;
pub const KEY_FRAGMENT_COUNT: usize = 5;
pub const MAX_STORY_FLAG_ID: u16 = 0x7FF;
pub const MAX_STORY_CHANGES_PER_MESSAGE: usize = 80;
pub const MAX_CONTAINER_ITEMS: usize = 8;
pub const MAX_CONTAINER_FLAGS: usize = 16;
pub const MAX_CONTAINER_GOLD: u32 = 9_999_999;
pub const MAX_TALK_CHOICE_INDEX: u8 = 63;

mod slot;
mod track;
mod wire;

pub use slot::PlayerSlot;
pub use track::{
    MAX_MOTION_EXTRAPOLATION_MS, MAX_TRACK_SAMPLES, MOTION_FRAMES_PER_MS, StateTrack,
    TRACK_HISTORY_MS, TRACK_RESTART_GAP_MS, TRACK_RESTART_TICK_JUMP, TrackSample,
};
pub use wire::{
    BuildGate, CombatEvent, CutsceneSkipAsk, DialogueLine, DialogueUpdate, EFFECT_ATTACH_BYTES,
    EFFECT_PARAM_BYTES, ENEMY_GROUP_COUNT, EffectPlay, EffectStop, EnemyDrop, EnemyGone,
    EnemyGoneReason, EnemyHit, EnemyId, EnemyKilled, EnemySnapshot, EnemySpawned, EnemyState,
    EnemyWordDrop, FadeSync, FollowPhase, GameReadiness, Hello, MAX_DIALOGUE_LINE_INDEX,
    MAX_DISPLAY_NAME_BYTES, MAX_EFFECT_BONE_BYTES, MAX_EFFECT_NAME_BYTES, MAX_ENEMIES_PER_SNAPSHOT,
    MAX_ENEMY_SNAPSHOT_PARTS, MAX_FADE_SECONDS, MAX_HIT_DAMAGE, MAX_HP, MAX_MESSAGE_BYTES,
    MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MAX_MUSIC_FADE_SECONDS, MAX_MUSIC_POSITION_MS,
    MAX_MUSIC_VOLUME_ABS, MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT, MAX_PHASE_POINT,
    MAX_PLAYERS, MAX_POSITION_ABS, MAX_SKY_AREA, MAX_SKY_ASSET_NAME_BYTES, MAX_SKY_BLEND_SECONDS,
    MAX_SKY_RECORD_BYTES, MAX_SKY_RECORD_CHUNK_BYTES, MAX_SOUND_NAME_BYTES, MAX_TALK_ID,
    MAX_WEAPON_ID, MAX_WEAPON_LEVEL, MAX_YAW_ABS_RADIANS, MUSIC_LAYER_COUNT, Message, MotionState,
    MusicState, NO_MUSIC_TRACK, NPC_UNGROUPED, NpcId, NpcSnapshot, NpcState, NpcTalk,
    PROTOCOL_VERSION, PartnerActivity, PartnerNotice, PathInfo, Ping, PlayerHit, PlayerSettings,
    PlayerState, Pong, Reject, RejectReason, SessionOptions, SkyAssets, SkyRecordChunk, SkyState,
    SoundPlay, WeaponState, Welcome, WireError, decode, encode,
};
pub use wire::{
    ContainerFlag, ContainerItem, ContainerOpened, MAX_CONTAINER_FLAGS, MAX_CONTAINER_GOLD,
    MAX_CONTAINER_ITEMS,
};
pub use wire::{
    DEFAULT_ENEMY_DAMAGE_PERCENT, DEFAULT_ENEMY_HEALTH_PERCENT, MAX_ENEMY_DAMAGE_PERCENT,
    MAX_ENEMY_HEALTH_PERCENT,
};
pub use wire::{
    DoorAction, DoorChange, DoorReport, DoorState, MAX_DOOR_LAYOUT_ID, MAX_DOORS_PER_REPORT,
};
pub use wire::{ENDING_ACCOMPLISHED_BYTES, ENDING_COSTUME_BYTES, EndingRecords};
pub use wire::{ItemPickup, MAX_ITEM_COUNT, MAX_ITEM_ID, MAX_WORD_ID, PickupSource};
pub use wire::{
    KEY_FRAGMENT_COUNT, MAX_STORY_CHANGES_PER_MESSAGE, MAX_STORY_FLAG_ID, MAX_TALK_CHOICE_INDEX,
    STORY_ARRAY_BYTES, StoryArray, StoryArrayPart, StoryChange, StoryChanges, StoryDigest,
    StoryRequest, StoryWrite, TalkChoice, TalkChoiceKind,
};
pub use wire::{LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME, LobbyReady, LobbyStart, MAX_LOBBY_SLOT};
pub use wire::{
    MAX_SCRIPT_MESSAGE_NUMBER, MAX_SCRIPT_ROLL_ORDINAL, ScriptMessage, ScriptRoll, ScriptRollValue,
};
pub use wire::{
    MAX_STORY_CALL_ARGS, MAX_STORY_CALL_NAME_BYTES, MAX_STORY_CALL_TEXT_BYTES, StoryCall,
    StoryCallArg, StoryRefusalReason, StoryRefused,
};

mod slot;
mod track;
mod wire;

pub use slot::PlayerSlot;
pub use track::{
    MAX_MOTION_EXTRAPOLATION_MS, MAX_TRACK_SAMPLES, MOTION_FRAMES_PER_MS, StateTrack,
    TRACK_HISTORY_MS, TRACK_RESTART_GAP_MS, TRACK_RESTART_TICK_JUMP, TrackSample,
};
pub use wire::{
    BuildGate, CombatEvent, DialogueLine, DialogueUpdate, EFFECT_ATTACH_BYTES, EFFECT_PARAM_BYTES,
    ENEMY_GROUP_COUNT, EffectPlay, EffectStop, EnemyGone, EnemyGoneReason, EnemyId, EnemySnapshot,
    EnemySpawned, EnemyState, FadeSync, FollowPhase, GameReadiness, Hello, MAX_DIALOGUE_LINE_INDEX,
    MAX_DISPLAY_NAME_BYTES, MAX_EFFECT_BONE_BYTES, MAX_EFFECT_NAME_BYTES, MAX_ENEMIES_PER_SNAPSHOT,
    MAX_ENEMY_SNAPSHOT_PARTS, MAX_FADE_SECONDS, MAX_HP, MAX_MESSAGE_BYTES, MAX_MOTION_FRAME,
    MAX_MOTION_SPEED_ABS, MAX_MUSIC_FADE_SECONDS, MAX_MUSIC_POSITION_MS, MAX_MUSIC_VOLUME_ABS,
    MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT, MAX_PHASE_POINT, MAX_PLAYERS, MAX_POSITION_ABS,
    MAX_SKY_AREA, MAX_SKY_ASSET_NAME_BYTES, MAX_SKY_BLEND_SECONDS, MAX_SKY_RECORD_BYTES,
    MAX_SKY_RECORD_CHUNK_BYTES, MAX_SOUND_NAME_BYTES, MAX_TALK_ID, MAX_WEAPON_ID, MAX_WEAPON_LEVEL,
    MAX_YAW_ABS_RADIANS, MUSIC_LAYER_COUNT, Message, MotionState, MusicState, NO_MUSIC_TRACK,
    NPC_UNGROUPED, NpcId, NpcSnapshot, NpcState, NpcTalk, PROTOCOL_VERSION, PartnerActivity,
    PartnerNotice, PathInfo, Ping, PlayerSettings, PlayerState, Pong, Reject, RejectReason,
    SessionOptions, SkyAssets, SkyRecordChunk, SkyState, SoundPlay, WeaponState, Welcome,
    WireError, decode, encode,
};
pub use wire::{ItemPickup, MAX_ITEM_COUNT, MAX_ITEM_ID, PickupSource};

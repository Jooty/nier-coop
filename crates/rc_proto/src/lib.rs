mod slot;
mod track;
mod wire;

pub use slot::PlayerSlot;
pub use track::{
    MAX_MOTION_EXTRAPOLATION_MS, MAX_TRACK_SAMPLES, MOTION_FRAMES_PER_MS, StateTrack,
    TRACK_HISTORY_MS, TRACK_RESTART_GAP_MS, TRACK_RESTART_TICK_JUMP, TrackSample,
};
pub use wire::{
    BuildGate, Challenge, CombatEvent, DialogueLine, DialogueUpdate, EFFECT_ATTACH_BYTES,
    EFFECT_PARAM_BYTES, ENEMY_GROUP_COUNT, EffectPlay, EffectStop, EnemyGone, EnemyGoneReason,
    EnemyId, EnemySnapshot, EnemySpawned, EnemyState, FollowPhase, GameReadiness, Hello,
    MAX_DIALOGUE_LINE_INDEX, MAX_DISPLAY_NAME_BYTES, MAX_EFFECT_BONE_BYTES, MAX_EFFECT_NAME_BYTES,
    MAX_ENEMIES_PER_SNAPSHOT, MAX_ENEMY_SNAPSHOT_PARTS, MAX_HP, MAX_MESSAGE_BYTES,
    MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MAX_MUSIC_FADE_SECONDS, MAX_MUSIC_POSITION_MS,
    MAX_MUSIC_VOLUME_ABS, MAX_PHASE_POINT, MAX_PLAYERS, MAX_POSITION_ABS, MAX_SKY_AREA,
    MAX_SKY_BLEND_SECONDS, MAX_SOUND_NAME_BYTES, MAX_TALK_ID, MAX_WEAPON_ID, MAX_WEAPON_LEVEL,
    MAX_YAW_ABS_RADIANS, MUSIC_LAYER_COUNT, Message, MotionState, MusicState, NO_MUSIC_TRACK,
    PROTOCOL_VERSION, PartnerActivity, PartnerNotice, PathInfo, Ping, PlayerSettings, PlayerState,
    Pong, Proof, Reject, RejectReason, SessionOptions, SkyState, SoundPlay, WeaponState, Welcome,
    WireError, decode, encode,
};
pub use wire::{ItemPickup, MAX_ITEM_COUNT, MAX_ITEM_ID, PickupSource};

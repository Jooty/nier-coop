use serde::{Deserialize, Serialize};

use crate::wire::{
    BuildGate, CombatEvent, DialogueUpdate, EffectPlay, EffectStop, EnemyGone, EnemySnapshot,
    EnemySpawned, FollowPhase, GameReadiness, ItemPickup, MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS,
    MusicState, PartnerNotice, PlayerSettings, PlayerState, SessionOptions, SkyState, SoundPlay,
    WireError,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    State(PlayerState),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Challenge(Challenge),
    Proof(Proof),
    PathInfo(PathInfo),
    Combat(CombatEvent),
    FollowPhase(FollowPhase),
    EnemySnapshot(EnemySnapshot),
    EnemySpawned(EnemySpawned),
    EnemyGone(EnemyGone),
    SessionOptions(SessionOptions),
    Readiness(GameReadiness),
    Notice(PartnerNotice),
    Effect(EffectPlay),
    Sound(SoundPlay),
    EffectStop(EffectStop),
    Settings(PlayerSettings),
    Music(MusicState),
    Pickup(ItemPickup),
    Sky(SkyState),
    Dialogue(DialogueUpdate),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hello {
    pub version: u16,
    pub build: BuildGate,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Welcome {
    pub version: u16,
    pub slot: u8,
    pub host_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    VersionMismatch,
    BuildMismatch,
    SessionFull,
    Malformed,
    PasswordRequired,
    WrongPassword,
    TooManyAttempts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reject {
    pub reason: RejectReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Challenge {
    pub nonce: [u8; 16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    pub mac: [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathInfo {
    pub rtt_micros: u32,
    pub direct: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub nonce: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub nonce: u32,
}

impl Message {
    pub fn is_latest_wins(&self) -> bool {
        matches!(self, Message::State(_) | Message::EnemySnapshot(_))
    }

    /// Game messages a session passes between the two games untouched, as opposed to the
    /// handshake and keepalive messages the session answers itself.
    pub fn is_relayed(&self) -> bool {
        matches!(
            self,
            Message::Combat(_)
                | Message::FollowPhase(_)
                | Message::EnemySnapshot(_)
                | Message::EnemySpawned(_)
                | Message::EnemyGone(_)
                | Message::SessionOptions(_)
                | Message::Readiness(_)
                | Message::Notice(_)
                | Message::Effect(_)
                | Message::Sound(_)
                | Message::EffectStop(_)
                | Message::Music(_)
                | Message::Pickup(_)
                | Message::Sky(_)
                | Message::Dialogue(_)
        )
    }

    pub fn validate(&self) -> Result<(), WireError> {
        match self {
            Message::Hello(hello) => validate_display_name(&hello.name),
            Message::Welcome(welcome) => {
                if welcome.slot >= MAX_PLAYERS {
                    return Err(WireError::OutOfRange { field: "slot" });
                }
                validate_display_name(&welcome.host_name)
            }
            Message::State(state) => state.validate(),
            Message::FollowPhase(follow) => follow.validate(),
            Message::EnemySnapshot(snapshot) => snapshot.validate(),
            Message::EnemySpawned(spawned) => spawned.enemy.validate(),
            Message::EnemyGone(gone) => gone.id.validate(),
            Message::Effect(effect) => effect.validate(),
            Message::Sound(sound) => sound.validate(),
            Message::Music(music) => music.validate(),
            Message::Pickup(pickup) => pickup.validate(),
            Message::Sky(sky) => sky.validate(),
            Message::Dialogue(update) => update.validate(),
            Message::Reject(_)
            | Message::Ping(_)
            | Message::Pong(_)
            | Message::Bye
            | Message::Challenge(_)
            | Message::Proof(_)
            | Message::PathInfo(_)
            | Message::Combat(_)
            | Message::SessionOptions(_)
            | Message::Readiness(_)
            | Message::Notice(_)
            | Message::EffectStop(_)
            | Message::Settings(_) => Ok(()),
        }
    }
}

fn validate_display_name(name: &str) -> Result<(), WireError> {
    if name.is_empty() || name.len() > MAX_DISPLAY_NAME_BYTES || name.chars().any(char::is_control)
    {
        return Err(WireError::OutOfRange { field: "name" });
    }
    Ok(())
}

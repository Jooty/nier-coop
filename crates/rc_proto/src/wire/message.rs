use serde::{Deserialize, Serialize};

use crate::wire::{
    BuildGate, CombatEvent, ContainerOpened, CutsceneSkipAsk, DialogueUpdate, DoorChange,
    DoorReport, EffectPlay, EffectStop, EnemyDrop, EnemyGone, EnemyHit, EnemyKilled, EnemySnapshot,
    EnemySpawned, EnemyWordDrop, FadeSync, FollowPhase, GameReadiness, ItemPickup,
    MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, MusicState, NpcSnapshot, NpcTalk, PartnerNotice,
    PlayerHit, PlayerSettings, PlayerState, SessionOptions, SkyAssets, SkyRecordChunk, SkyState,
    SoundPlay, StoryArrayPart, StoryCall, StoryChanges, StoryDigest, StoryRequest, StoryWrite,
    TalkChoice, WireError,
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
    NpcSnapshot(NpcSnapshot),
    SkyRecord(SkyRecordChunk),
    NpcTalk(NpcTalk),
    SkyAssets(SkyAssets),
    Fade(FadeSync),
    EnemyHit(EnemyHit),
    PlayerHit(PlayerHit),
    EnemyKilled(EnemyKilled),
    EnemyDrop(EnemyDrop),
    DoorChange(DoorChange),
    DoorRequest(DoorChange),
    DoorReport(DoorReport),
    CutsceneSkip(CutsceneSkipAsk),
    StoryRequest(StoryRequest),
    StoryPart(StoryArrayPart),
    StoryChanges(StoryChanges),
    StoryDigest(StoryDigest),
    EnemyWordDrop(EnemyWordDrop),
    ContainerOpened(ContainerOpened),
    StoryWrite(StoryWrite),
    TalkChoice(TalkChoice),
    StoryTrigger(StoryCall),
    StoryRun(StoryCall),
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reject {
    pub reason: RejectReason,
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
        matches!(
            self,
            Message::State(_) | Message::EnemySnapshot(_) | Message::NpcSnapshot(_)
        )
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
                | Message::NpcSnapshot(_)
                | Message::NpcTalk(_)
                | Message::Music(_)
                | Message::Pickup(_)
                | Message::Sky(_)
                | Message::SkyRecord(_)
                | Message::SkyAssets(_)
                | Message::Dialogue(_)
                | Message::Fade(_)
                | Message::EnemyHit(_)
                | Message::PlayerHit(_)
                | Message::EnemyKilled(_)
                | Message::EnemyDrop(_)
                | Message::DoorChange(_)
                | Message::DoorRequest(_)
                | Message::DoorReport(_)
                | Message::CutsceneSkip(_)
                | Message::StoryRequest(_)
                | Message::StoryPart(_)
                | Message::StoryChanges(_)
                | Message::StoryDigest(_)
                | Message::EnemyWordDrop(_)
                | Message::ContainerOpened(_)
                | Message::StoryWrite(_)
                | Message::TalkChoice(_)
                | Message::StoryTrigger(_)
                | Message::StoryRun(_)
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
            Message::NpcSnapshot(snapshot) => snapshot.validate(),
            Message::NpcTalk(talk) => talk.validate(),
            Message::Music(music) => music.validate(),
            Message::Pickup(pickup) => pickup.validate(),
            Message::Sky(sky) => sky.validate(),
            Message::SkyRecord(chunk) => chunk.validate(),
            Message::SkyAssets(assets) => assets.validate(),
            Message::Dialogue(update) => update.validate(),
            Message::Fade(fade) => fade.validate(),
            Message::EnemyHit(hit) => hit.validate(),
            Message::PlayerHit(hit) => hit.validate(),
            Message::EnemyKilled(killed) => killed.id.validate(),
            Message::EnemyDrop(drop) => drop.validate(),
            Message::EnemyWordDrop(drop) => drop.validate(),
            Message::DoorChange(change) | Message::DoorRequest(change) => change.validate(),
            Message::DoorReport(report) => report.validate(),
            Message::ContainerOpened(opened) => opened.validate(),
            Message::StoryPart(part) => part.validate(),
            Message::StoryChanges(changes) => changes.validate(),
            Message::StoryWrite(write) => write.validate(),
            Message::TalkChoice(choice) => choice.validate(),
            Message::StoryTrigger(call) | Message::StoryRun(call) => call.validate(),
            Message::Reject(_)
            | Message::Ping(_)
            | Message::Pong(_)
            | Message::Bye
            | Message::PathInfo(_)
            | Message::Combat(_)
            | Message::SessionOptions(_)
            | Message::Readiness(_)
            | Message::Notice(_)
            | Message::EffectStop(_)
            | Message::Settings(_)
            | Message::CutsceneSkip(_)
            | Message::StoryRequest(_)
            | Message::StoryDigest(_) => Ok(()),
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

use nc_wire::{GameMessage, Hello, LinkPayload, Ping, Pong, Reject, Welcome};
use serde::{Deserialize, Serialize};

use crate::wire::{
    AttackGaugeHit, AttackGaugeState, BlockHeld, BlockMoved, BodyPackName, BombBlast,
    BulletEffectPlay, BulletFlight, CallbackFired, CartRide, ChoiceHover, CombatEvent,
    ContainerOpened, CutsceneSkipAsk, DesyncDigest, DialogueUpdate, DoorChange, DoorReport,
    EffectPlay, EffectStop, EndingRecords, EnemyBodyGone, EnemyBulletFlight, EnemyCreation,
    EnemyDrop, EnemyGone, EnemyHit, EnemyKilled, EnemySnapshot, EnemySpawned, EnemyWordDrop,
    FadeSync, FightCamera, FightPromptCount, FightRadioWaitEnded, FightRequest, FightRequestAnswer,
    FightStreamEntry, FightTape, FightWindowMark, FollowPhase, GameReadiness, GatherAnswer,
    GatherForScene, ItemCounts, ItemPickup, ItemTake, LiftCall, LiftCallAnswer, LobbyBody,
    LobbyReady, LobbyStart, MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, MessageHandlerRan, MusicState,
    NpcSnapshot, NpcTalk, PROTOCOL_VERSION, PartnerNotice, PausedHits, PlayerHit, PlayerSettings,
    PlayerState, PowerModelEnded, PowerModelMotion, PowerModelState, PrologueLevel, SceneFollow,
    ScriptMessage, ScriptRoll, SessionOptions, SharedFunctionRan, SkyAssets, SkyRecordChunk,
    SkyState, SoundPlay, StoryArrayPart, StoryCall, StoryChanges, StoryDigest, StoryJump,
    StoryRefused, StoryRequest, StoryWrite, TalkChoice, TeleportHere, TutorialPrompt,
    VoiceCharacter, WallBroken, WireError, WorldEffectMove, WorldEffectPlay,
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
    ScriptRoll(ScriptRoll),
    StoryRefused(StoryRefused),
    ScriptMessage(ScriptMessage),
    EndingRecords(EndingRecords),
    LobbyReady(LobbyReady),
    LobbyStart(LobbyStart),
    PartnerBody(BodyPackName),
    TutorialPrompt(TutorialPrompt),
    BookEffect(EffectPlay),
    BookSound(SoundPlay),
    BulletEffect(BulletEffectPlay),
    PrologueLevel(PrologueLevel),
    PowerModel(PowerModelState),
    BulletFlight(BulletFlight),
    SceneFollow(SceneFollow),
    CallbackFired(CallbackFired),
    Desync(DesyncDigest),
    EnemyBulletFlight(EnemyBulletFlight),
    PausedHits(PausedHits),
    EnemyBodyGone(EnemyBodyGone),
    PowerMotion(PowerModelMotion),
    PowerEnded(PowerModelEnded),
    WorldEffect(WorldEffectPlay),
    WorldEffectMove(WorldEffectMove),
    VoiceCharacter(VoiceCharacter),
    StoryJump(StoryJump),
    LobbyBody(LobbyBody),
    QuitGame,
    ItemCounts(ItemCounts),
    ItemTake(ItemTake),
    TeleportHere(TeleportHere),
    EnemyCreation(EnemyCreation),
    PartnerBook(bool),
    BlockMoved(BlockMoved),
    BlockHeld(BlockHeld),
    DummyHit(StoryCall),
    WatchedCallbackFired(CallbackFired),
    SharedFunctionRan(SharedFunctionRan),
    MessageHandlerRan(MessageHandlerRan),
    FightStream(FightStreamEntry),
    FightWindow(FightWindowMark),
    FightRequest(FightRequest),
    FightTape(FightTape),
    FightRequestAnswer(FightRequestAnswer),
    FightRadioWait(FightRadioWaitEnded),
    FightPromptCount(FightPromptCount),
    AttackGauge(AttackGaugeState),
    AttackGaugeHit(AttackGaugeHit),
    GatherForScene(GatherForScene),
    GatherAnswer(GatherAnswer),
    ChoiceHover(ChoiceHover),
    WallBroken(WallBroken),
    BombBlast(BombBlast),
    CartRide(CartRide),
    LiftCall(LiftCall),
    LiftCallAnswer(LiftCallAnswer),
    FightCamera(FightCamera),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathInfo {
    pub rtt_micros: u32,
    pub direct: bool,
}

impl Message {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Message::Hello(_) => "Hello",
            Message::Welcome(_) => "Welcome",
            Message::Reject(_) => "Reject",
            Message::State(_) => "State",
            Message::Ping(_) => "Ping",
            Message::Pong(_) => "Pong",
            Message::Bye => "Bye",
            Message::PathInfo(_) => "PathInfo",
            Message::Combat(_) => "Combat",
            Message::FollowPhase(_) => "FollowPhase",
            Message::EnemySnapshot(_) => "EnemySnapshot",
            Message::EnemySpawned(_) => "EnemySpawned",
            Message::EnemyCreation(_) => "EnemyCreation",
            Message::EnemyGone(_) => "EnemyGone",
            Message::SessionOptions(_) => "SessionOptions",
            Message::Readiness(_) => "Readiness",
            Message::Notice(_) => "Notice",
            Message::Effect(_) => "Effect",
            Message::Sound(_) => "Sound",
            Message::EffectStop(_) => "EffectStop",
            Message::Settings(_) => "Settings",
            Message::Music(_) => "Music",
            Message::Pickup(_) => "Pickup",
            Message::Sky(_) => "Sky",
            Message::Dialogue(_) => "Dialogue",
            Message::NpcSnapshot(_) => "NpcSnapshot",
            Message::SkyRecord(_) => "SkyRecord",
            Message::NpcTalk(_) => "NpcTalk",
            Message::SkyAssets(_) => "SkyAssets",
            Message::Fade(_) => "Fade",
            Message::EnemyHit(_) => "EnemyHit",
            Message::PlayerHit(_) => "PlayerHit",
            Message::EnemyKilled(_) => "EnemyKilled",
            Message::EnemyDrop(_) => "EnemyDrop",
            Message::DoorChange(_) => "DoorChange",
            Message::DoorRequest(_) => "DoorRequest",
            Message::DoorReport(_) => "DoorReport",
            Message::CutsceneSkip(_) => "CutsceneSkip",
            Message::StoryRequest(_) => "StoryRequest",
            Message::StoryPart(_) => "StoryPart",
            Message::StoryChanges(_) => "StoryChanges",
            Message::StoryDigest(_) => "StoryDigest",
            Message::EnemyWordDrop(_) => "EnemyWordDrop",
            Message::ContainerOpened(_) => "ContainerOpened",
            Message::StoryWrite(_) => "StoryWrite",
            Message::TalkChoice(_) => "TalkChoice",
            Message::StoryTrigger(_) => "StoryTrigger",
            Message::StoryRun(_) => "StoryRun",
            Message::ScriptRoll(_) => "ScriptRoll",
            Message::StoryRefused(_) => "StoryRefused",
            Message::ScriptMessage(_) => "ScriptMessage",
            Message::EndingRecords(_) => "EndingRecords",
            Message::LobbyReady(_) => "LobbyReady",
            Message::LobbyStart(_) => "LobbyStart",
            Message::PartnerBody(_) => "PartnerBody",
            Message::TutorialPrompt(_) => "TutorialPrompt",
            Message::BookEffect(_) => "BookEffect",
            Message::BookSound(_) => "BookSound",
            Message::BulletEffect(_) => "BulletEffect",
            Message::PrologueLevel(_) => "PrologueLevel",
            Message::PowerModel(_) => "PowerModel",
            Message::BulletFlight(_) => "BulletFlight",
            Message::SceneFollow(_) => "SceneFollow",
            Message::CallbackFired(_) => "CallbackFired",
            Message::Desync(_) => "Desync",
            Message::EnemyBulletFlight(_) => "EnemyBulletFlight",
            Message::PausedHits(_) => "PausedHits",
            Message::EnemyBodyGone(_) => "EnemyBodyGone",
            Message::PowerMotion(_) => "PowerMotion",
            Message::PowerEnded(_) => "PowerEnded",
            Message::WorldEffect(_) => "WorldEffect",
            Message::WorldEffectMove(_) => "WorldEffectMove",
            Message::VoiceCharacter(_) => "VoiceCharacter",
            Message::StoryJump(_) => "StoryJump",
            Message::LobbyBody(_) => "LobbyBody",
            Message::ItemCounts(_) => "ItemCounts",
            Message::ItemTake(_) => "ItemTake",
            Message::TeleportHere(_) => "TeleportHere",
            Message::GatherForScene(_) => "GatherForScene",
            Message::GatherAnswer(_) => "GatherAnswer",
            Message::QuitGame => "QuitGame",
            Message::PartnerBook(_) => "PartnerBook",
            Message::BlockMoved(_) => "BlockMoved",
            Message::BlockHeld(_) => "BlockHeld",
            Message::DummyHit(_) => "DummyHit",
            Message::WatchedCallbackFired(_) => "WatchedCallbackFired",
            Message::SharedFunctionRan(_) => "SharedFunctionRan",
            Message::MessageHandlerRan(_) => "MessageHandlerRan",
            Message::FightStream(_) => "FightStream",
            Message::FightWindow(_) => "FightWindow",
            Message::FightRequest(_) => "FightRequest",
            Message::FightTape(_) => "FightTape",
            Message::FightRequestAnswer(_) => "FightRequestAnswer",
            Message::FightRadioWait(_) => "FightRadioWait",
            Message::FightPromptCount(_) => "FightPromptCount",
            Message::AttackGauge(_) => "AttackGauge",
            Message::AttackGaugeHit(_) => "AttackGaugeHit",
            Message::ChoiceHover(_) => "ChoiceHover",
            Message::WallBroken(_) => "WallBroken",
            Message::BombBlast(_) => "BombBlast",
            Message::CartRide(_) => "CartRide",
            Message::LiftCall(_) => "LiftCall",
            Message::LiftCallAnswer(_) => "LiftCallAnswer",
            Message::FightCamera(_) => "FightCamera",
        }
    }

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
                | Message::EnemyCreation(_)
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
                | Message::ScriptRoll(_)
                | Message::StoryRefused(_)
                | Message::ScriptMessage(_)
                | Message::EndingRecords(_)
                | Message::LobbyReady(_)
                | Message::LobbyStart(_)
                | Message::PartnerBody(_)
                | Message::TutorialPrompt(_)
                | Message::BookEffect(_)
                | Message::BookSound(_)
                | Message::BulletEffect(_)
                | Message::PrologueLevel(_)
                | Message::PowerModel(_)
                | Message::BulletFlight(_)
                | Message::SceneFollow(_)
                | Message::CallbackFired(_)
                | Message::Desync(_)
                | Message::EnemyBulletFlight(_)
                | Message::PausedHits(_)
                | Message::EnemyBodyGone(_)
                | Message::PowerMotion(_)
                | Message::PowerEnded(_)
                | Message::WorldEffect(_)
                | Message::WorldEffectMove(_)
                | Message::VoiceCharacter(_)
                | Message::StoryJump(_)
                | Message::LobbyBody(_)
                | Message::ItemCounts(_)
                | Message::ItemTake(_)
                | Message::TeleportHere(_)
                | Message::GatherForScene(_)
                | Message::GatherAnswer(_)
                | Message::PartnerBook(_)
                | Message::BlockMoved(_)
                | Message::BlockHeld(_)
                | Message::DummyHit(_)
                | Message::WatchedCallbackFired(_)
                | Message::SharedFunctionRan(_)
                | Message::MessageHandlerRan(_)
                | Message::FightStream(_)
                | Message::FightWindow(_)
                | Message::FightRequest(_)
                | Message::FightTape(_)
                | Message::FightRequestAnswer(_)
                | Message::FightRadioWait(_)
                | Message::FightPromptCount(_)
                | Message::AttackGauge(_)
                | Message::AttackGaugeHit(_)
                | Message::ChoiceHover(_)
                | Message::WallBroken(_)
                | Message::BombBlast(_)
                | Message::CartRide(_)
                | Message::LiftCall(_)
                | Message::LiftCallAnswer(_)
                | Message::FightCamera(_)
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
            Message::EnemyCreation(creation) => creation.validate(),
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
            Message::StoryTrigger(call) | Message::StoryRun(call) | Message::DummyHit(call) => {
                call.validate()
            }
            Message::ScriptRoll(roll) => roll.validate(),
            Message::StoryRefused(refused) => refused.validate(),
            Message::ScriptMessage(message) => message.validate(),
            Message::EndingRecords(records) => records.validate(),
            Message::LobbyReady(ready) => ready.validate(),
            Message::LobbyStart(start) => start.validate(),
            Message::PartnerBody(pack) => pack.validate(),
            Message::TutorialPrompt(prompt) => prompt.validate(),
            Message::BookEffect(effect) => effect.validate(),
            Message::BookSound(sound) => sound.validate(),
            Message::BulletEffect(bullet) => bullet.validate(),
            Message::PrologueLevel(level) => level.validate(),
            Message::PowerModel(power) => power.validate(),
            Message::BulletFlight(flight) => flight.validate(),
            Message::SceneFollow(follow) => follow.validate(),
            Message::CallbackFired(fired) | Message::WatchedCallbackFired(fired) => {
                fired.validate()
            }
            Message::Desync(digest) => digest.validate(),
            Message::EnemyBulletFlight(flight) => flight.validate(),
            Message::PausedHits(hits) => hits.validate(),
            Message::EnemyBodyGone(gone) => gone.id.validate(),
            Message::PowerMotion(motion) => motion.validate(),
            Message::PowerEnded(ended) => ended.validate(),
            Message::WorldEffect(effect) => effect.validate(),
            Message::WorldEffectMove(moved) => moved.validate(),
            Message::VoiceCharacter(voice) => voice.validate(),
            Message::StoryJump(_) => Ok(()),
            Message::LobbyBody(_) => Ok(()),
            Message::ItemCounts(counts) => counts.validate(),
            Message::ItemTake(take) => take.validate(),
            Message::TeleportHere(here) => here.validate(),
            Message::GatherForScene(gather) => gather.validate(),
            Message::BlockMoved(moved) => moved.validate(),
            Message::BlockHeld(held) => held.validate(),
            Message::SharedFunctionRan(ran) => ran.validate(),
            Message::MessageHandlerRan(ran) => ran.validate(),
            Message::FightStream(entry) => entry.validate(),
            Message::FightWindow(mark) => mark.validate(),
            Message::FightRequest(request) => request.validate(),
            Message::FightTape(tape) => tape.validate(),
            Message::FightRequestAnswer(answer) => answer.validate(),
            Message::FightRadioWait(ended) => ended.validate(),
            Message::FightPromptCount(count) => count.validate(),
            Message::AttackGauge(state) => state.validate(),
            Message::AttackGaugeHit(hit) => hit.validate(),
            Message::ChoiceHover(hover) => hover.validate(),
            Message::WallBroken(wall) => wall.validate(),
            Message::BombBlast(blast) => blast.validate(),
            Message::CartRide(ride) => ride.validate(),
            Message::LiftCall(call) => call.validate(),
            Message::LiftCallAnswer(answer) => answer.validate(),
            Message::FightCamera(camera) => camera.validate(),
            Message::Reject(_)
            | Message::GatherAnswer(_)
            | Message::Ping(_)
            | Message::Pong(_)
            | Message::Bye
            | Message::QuitGame
            | Message::PartnerBook(_)
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

impl GameMessage for Message {
    const PROTOCOL_VERSION: u16 = PROTOCOL_VERSION;
    const MAX_PLAYERS: u8 = MAX_PLAYERS;

    fn from_link(payload: LinkPayload) -> Self {
        match payload {
            LinkPayload::Hello(hello) => Message::Hello(hello),
            LinkPayload::Welcome(welcome) => Message::Welcome(welcome),
            LinkPayload::Reject(reject) => Message::Reject(reject),
            LinkPayload::Ping(ping) => Message::Ping(ping),
            LinkPayload::Pong(pong) => Message::Pong(pong),
            LinkPayload::Bye => Message::Bye,
        }
    }

    fn into_link(self) -> Result<LinkPayload, Self> {
        match self {
            Message::Hello(hello) => Ok(LinkPayload::Hello(hello)),
            Message::Welcome(welcome) => Ok(LinkPayload::Welcome(welcome)),
            Message::Reject(reject) => Ok(LinkPayload::Reject(reject)),
            Message::Ping(ping) => Ok(LinkPayload::Ping(ping)),
            Message::Pong(pong) => Ok(LinkPayload::Pong(pong)),
            Message::Bye => Ok(LinkPayload::Bye),
            other => Err(other),
        }
    }

    fn is_relayed(&self) -> bool {
        Message::is_relayed(self)
    }

    fn is_latest_wins(&self) -> bool {
        Message::is_latest_wins(self)
    }

    fn ordered_tick(&self) -> Option<u32> {
        match self {
            Message::State(state) => Some(state.tick),
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), WireError> {
        Message::validate(self)
    }
}

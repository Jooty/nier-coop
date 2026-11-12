use automata_game::enemies::{
    EnemyHits, EnemyId, EnemyRosterPart, EnemySpawn, GeneratorStates, LiveGenerators,
};
use automata_game::enemy_lead::EnemyLead;
use automata_game::travel::TeleportRequest;
use nc_wire::{
    GameMessage, GameReadiness, Hello, LinkPayload, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart, Member, Ping, Pong, Reject, SessionRules, Welcome,
};
use serde::{Deserialize, Serialize};

use crate::wire::{
    CharacterHello, MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, PROTOCOL_VERSION, PathInfo, PlayerState,
    WireError,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    Ping(Ping),
    Pong(Pong),
    Bye,
    Character(CharacterHello),
    State(PlayerState),
    PathInfo(PathInfo),
    LobbyReady(LobbyReady),
    LobbyStart(LobbyStart),
    LobbyCharacter(LobbyCharacter),
    Readiness(GameReadiness),
    Teleport(TeleportRequest),
    EnemyRoster(EnemyRosterPart),
    EnemyHits(EnemyHits),
    EnemyDefeated(EnemyId),
    EnemySpawned(EnemySpawn),
    EnemyGone(EnemyId),
    GeneratorStates(GeneratorStates),
    LiveGenerators(LiveGenerators),
    EnemyLead(EnemyLead),
    Member(Member),
    MemberLeft { slot: u8 },
    Rules(SessionRules),
}

impl Message {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Message::Hello(_) => "Hello",
            Message::Welcome(_) => "Welcome",
            Message::Reject(_) => "Reject",
            Message::Ping(_) => "Ping",
            Message::Pong(_) => "Pong",
            Message::Bye => "Bye",
            Message::Character(_) => "Character",
            Message::State(_) => "State",
            Message::PathInfo(_) => "PathInfo",
            Message::LobbyReady(_) => "LobbyReady",
            Message::LobbyStart(_) => "LobbyStart",
            Message::LobbyCharacter(_) => "LobbyCharacter",
            Message::Readiness(_) => "Readiness",
            Message::Teleport(_) => "Teleport",
            Message::EnemyRoster(_) => "EnemyRoster",
            Message::EnemyHits(_) => "EnemyHits",
            Message::EnemyDefeated(_) => "EnemyDefeated",
            Message::EnemySpawned(_) => "EnemySpawned",
            Message::EnemyGone(_) => "EnemyGone",
            Message::GeneratorStates(_) => "GeneratorStates",
            Message::LiveGenerators(_) => "LiveGenerators",
            Message::EnemyLead(_) => "EnemyLead",
            Message::Member(_) => "Member",
            Message::MemberLeft { .. } => "MemberLeft",
            Message::Rules(_) => "Rules",
        }
    }

    pub fn is_latest_wins(&self) -> bool {
        matches!(self, Message::State(_))
    }

    pub fn is_relayed(&self) -> bool {
        matches!(
            self,
            Message::Character(_)
                | Message::State(_)
                | Message::LobbyReady(_)
                | Message::LobbyStart(_)
                | Message::LobbyCharacter(_)
                | Message::Readiness(_)
                | Message::Teleport(_)
                | Message::EnemyRoster(_)
                | Message::EnemyHits(_)
                | Message::EnemyDefeated(_)
                | Message::EnemySpawned(_)
                | Message::EnemyGone(_)
                | Message::GeneratorStates(_)
                | Message::LiveGenerators(_)
                | Message::EnemyLead(_)
        )
    }

    pub fn validate(&self) -> Result<(), WireError> {
        match self {
            Message::Hello(hello) => validate_display_name(&hello.name),
            Message::Welcome(welcome) => {
                validate_slot(welcome.slot)?;
                validate_display_name(&welcome.host_name)
            }
            Message::State(state) => state.validate(),
            Message::LobbyReady(ready) => ready.validate(),
            Message::LobbyStart(start) => start.validate(),
            Message::Teleport(request) => {
                if request.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "point" })
                }
            }
            Message::EnemyRoster(part) => {
                if part.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "enemies" })
                }
            }
            Message::EnemyHits(hits) => {
                if hits.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "hits" })
                }
            }
            Message::EnemySpawned(spawn) => {
                if spawn.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "spawn" })
                }
            }
            Message::GeneratorStates(states) => {
                if states.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "states" })
                }
            }
            Message::LiveGenerators(live) => {
                if live.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange {
                        field: "generators",
                    })
                }
            }
            Message::EnemyLead(lead) => {
                if lead.is_valid() {
                    Ok(())
                } else {
                    Err(WireError::OutOfRange { field: "poses" })
                }
            }
            Message::Member(member) => {
                validate_slot(member.slot)?;
                validate_display_name(&member.name)
            }
            Message::MemberLeft { slot } => validate_slot(*slot),
            Message::Rules(rules) => validate_rules(rules),
            Message::Reject(_)
            | Message::Ping(_)
            | Message::Pong(_)
            | Message::Bye
            | Message::Character(_)
            | Message::PathInfo(_)
            | Message::LobbyCharacter(_)
            | Message::Readiness(_)
            | Message::EnemyDefeated(_)
            | Message::EnemyGone(_) => Ok(()),
        }
    }
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
            LinkPayload::Member(member) => Message::Member(member),
            LinkPayload::MemberLeft { slot } => Message::MemberLeft { slot },
            LinkPayload::Rules(rules) => Message::Rules(rules),
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
            Message::Member(member) => Ok(LinkPayload::Member(member)),
            Message::MemberLeft { slot } => Ok(LinkPayload::MemberLeft { slot }),
            Message::Rules(rules) => Ok(LinkPayload::Rules(rules)),
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
            Message::State(state) => Some(state.sequence),
            _ => None,
        }
    }

    fn validate(&self) -> Result<(), WireError> {
        Message::validate(self)
    }
}

impl LobbyMessage for Message {
    fn from_lobby(payload: LobbyPayload) -> Self {
        match payload {
            LobbyPayload::Ready(ready) => Message::LobbyReady(ready),
            LobbyPayload::Start(start) => Message::LobbyStart(start),
            LobbyPayload::Character(character) => Message::LobbyCharacter(character),
            LobbyPayload::GameReadiness(readiness) => Message::Readiness(readiness),
        }
    }

    fn into_lobby(self) -> Result<LobbyPayload, Self> {
        match self {
            Message::LobbyReady(ready) => Ok(LobbyPayload::Ready(ready)),
            Message::LobbyStart(start) => Ok(LobbyPayload::Start(start)),
            Message::LobbyCharacter(character) => Ok(LobbyPayload::Character(character)),
            Message::Readiness(readiness) => Ok(LobbyPayload::GameReadiness(readiness)),
            other => Err(other),
        }
    }
}

fn validate_slot(slot: u8) -> Result<(), WireError> {
    if slot >= MAX_PLAYERS {
        return Err(WireError::OutOfRange { field: "slot" });
    }
    Ok(())
}

fn validate_rules(rules: &SessionRules) -> Result<(), WireError> {
    if !(2..=MAX_PLAYERS).contains(&rules.max_players) {
        return Err(WireError::OutOfRange {
            field: "max_players",
        });
    }
    Ok(())
}

fn validate_display_name(name: &str) -> Result<(), WireError> {
    if name.is_empty() || name.len() > MAX_DISPLAY_NAME_BYTES || name.chars().any(char::is_control)
    {
        return Err(WireError::OutOfRange { field: "name" });
    }
    Ok(())
}

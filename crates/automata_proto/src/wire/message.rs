use nc_wire::{
    GameMessage, GameReadiness, Hello, LinkPayload, LobbyCharacter, LobbyMessage, LobbyPayload,
    LobbyReady, LobbyStart, Ping, Pong, Reject, Welcome,
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
            Message::LobbyReady(ready) => ready.validate(),
            Message::LobbyStart(start) => start.validate(),
            Message::Reject(_)
            | Message::Ping(_)
            | Message::Pong(_)
            | Message::Bye
            | Message::Character(_)
            | Message::PathInfo(_)
            | Message::LobbyCharacter(_)
            | Message::Readiness(_) => Ok(()),
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

fn validate_display_name(name: &str) -> Result<(), WireError> {
    if name.is_empty() || name.len() > MAX_DISPLAY_NAME_BYTES || name.chars().any(char::is_control)
    {
        return Err(WireError::OutOfRange { field: "name" });
    }
    Ok(())
}

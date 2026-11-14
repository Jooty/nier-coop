use serde::{Deserialize, Serialize};

use crate::{GameMessage, GameReadiness, WireError};

pub const LOBBY_MODE_NEW_GAME: u8 = 0;
pub const LOBBY_MODE_CONTINUE: u8 = 1;
pub const MAX_LOBBY_SLOT: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyReady {
    pub ready: bool,
    pub mode: u8,
    pub slot: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyStart {
    pub mode: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyCharacter {
    pub index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyPayload {
    Ready(LobbyReady),
    Start(LobbyStart),
    Character(LobbyCharacter),
    GameReadiness(GameReadiness),
}

/// What the two launchers say to each other before and around a run, the same under every game.
pub trait LobbyMessage: GameMessage {
    fn from_lobby(payload: LobbyPayload) -> Self;

    fn into_lobby(self) -> Result<LobbyPayload, Self>;
}

impl LobbyReady {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_mode(self.mode)?;
        if self.slot.is_some_and(|slot| slot > MAX_LOBBY_SLOT) {
            return Err(WireError::OutOfRange { field: "slot" });
        }
        Ok(())
    }
}

impl LobbyStart {
    pub fn validate(&self) -> Result<(), WireError> {
        validate_mode(self.mode)
    }
}

fn validate_mode(mode: u8) -> Result<(), WireError> {
    if mode > LOBBY_MODE_CONTINUE {
        return Err(WireError::OutOfRange { field: "mode" });
    }
    Ok(())
}

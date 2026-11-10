use serde::{Deserialize, Serialize};

use crate::wire::{PartnerBody, WireError};

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
pub struct LobbyBody {
    pub body: PartnerBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyCharacter {
    pub index: u8,
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

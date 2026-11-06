use serde::{Deserialize, Serialize};

use crate::wire::{BuildGate, MAX_DISPLAY_NAME_BYTES, MAX_PLAYERS, PlayerState, WireError};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Welcome(Welcome),
    Reject(Reject),
    State(PlayerState),
    Ping(Ping),
    Pong(Pong),
    Bye,
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
pub struct Ping {
    pub nonce: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub nonce: u32,
}

impl Message {
    pub fn is_latest_wins(&self) -> bool {
        matches!(self, Message::State(_))
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
            Message::Reject(_) | Message::Ping(_) | Message::Pong(_) | Message::Bye => Ok(()),
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

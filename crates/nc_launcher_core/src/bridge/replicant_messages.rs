use replicant_proto::{LobbyBody, Message, PathInfo};

use crate::bridge::{BridgeMessage, LauncherPayload};

impl BridgeMessage for Message {
    const LOBBY_PICKS_CHARACTERS: bool = false;

    fn kind_name(&self) -> &'static str {
        Message::kind_name(self)
    }

    fn path_info(rtt_micros: u32, direct: bool) -> Self {
        Message::PathInfo(PathInfo { rtt_micros, direct })
    }

    fn from_launcher(payload: LauncherPayload) -> Option<Self> {
        Some(match payload {
            LauncherPayload::Settings(settings) => Message::Settings(settings),
            LauncherPayload::SessionOptions(options) => Message::SessionOptions(options),
            LauncherPayload::LobbyBody(body) => Message::LobbyBody(LobbyBody { body }),
            LauncherPayload::QuitGame => Message::QuitGame,
        })
    }

    fn into_launcher(self) -> Result<LauncherPayload, Self> {
        match self {
            Message::Settings(settings) => Ok(LauncherPayload::Settings(settings)),
            Message::SessionOptions(options) => Ok(LauncherPayload::SessionOptions(options)),
            Message::LobbyBody(LobbyBody { body }) => Ok(LauncherPayload::LobbyBody(body)),
            Message::QuitGame => Ok(LauncherPayload::QuitGame),
            other => Err(other),
        }
    }
}

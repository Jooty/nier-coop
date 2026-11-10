use nc_wire::LobbyMessage;
use replicant_proto::{PartnerBody, PlayerSettings, SessionOptions};

/// What a launcher and a game say to each other beyond the lobby; a game's messages may carry none of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LauncherPayload {
    Settings(PlayerSettings),
    SessionOptions(SessionOptions),
    LobbyBody(PartnerBody),
    QuitGame,
}

pub(crate) trait BridgeMessage: LobbyMessage + 'static {
    fn kind_name(&self) -> &'static str;

    fn path_info(rtt_micros: u32, direct: bool) -> Self;

    fn from_launcher(payload: LauncherPayload) -> Option<Self>;

    fn into_launcher(self) -> Result<LauncherPayload, Self>;
}

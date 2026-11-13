use nc_wire::LobbyMessage;
use replicant_proto::{
    CampaignCard, PartnerBody, PlayerSettings, SeatPart, SeatReceipt, SessionOptions,
};

/// What a launcher and a game say to each other beyond the lobby; a game's messages may carry none of it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum LauncherPayload {
    Settings(PlayerSettings),
    SessionOptions(SessionOptions),
    LobbyBody(PartnerBody),
    QuitGame,
    CampaignCard(CampaignCard),
    SeatPart(SeatPart),
    SeatReceipt(SeatReceipt),
    GameSaved { succeeded: bool },
}

pub(crate) trait BridgeMessage: LobbyMessage + Send + 'static {
    const LOBBY_PICKS_CHARACTERS: bool;
    const CARRIES_CAMPAIGN_SAVES: bool;

    fn kind_name(&self) -> &'static str;

    fn path_info(rtt_micros: u32, direct: bool) -> Self;

    fn from_launcher(payload: LauncherPayload) -> Option<Self>;

    fn into_launcher(self) -> Result<LauncherPayload, Self>;
}

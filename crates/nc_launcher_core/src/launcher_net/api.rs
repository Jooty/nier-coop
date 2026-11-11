use replicant_proto::{LobbyReady, PartnerBody, PlayerSettings, SessionOptions};

use crate::{ConnectionReport, NetError, NetStatus};

pub trait LauncherNet: Send + Sync {
    fn host(&self) -> Result<String, NetError>;
    fn join(&self, ticket: &str) -> Result<(), NetError>;
    fn test_connection(&self, ticket: &str, seconds: u32) -> Result<ConnectionReport, NetError>;
    fn leave(&self);
    fn set_session_options(&self, _options: SessionOptions) {}
    fn set_player_settings(&self, _settings: PlayerSettings) {}
    fn send_lobby_ready(&self, _ready: LobbyReady) {}
    fn send_lobby_body(&self, _body: PartnerBody) {}
    fn start_lobby(&self, _mode: u8) {}
    fn ask_game_to_quit(&self) {}
    fn status(&self) -> NetStatus;
}

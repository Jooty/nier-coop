use rc_proto::{PlayerSettings, SessionOptions};

use crate::{ConnectionReport, NetError, NetStatus};

pub trait LauncherNet: Send + Sync {
    fn host(&self, password: Option<&str>) -> Result<String, NetError>;
    fn join(&self, ticket: &str, password: Option<&str>) -> Result<(), NetError>;
    fn test_connection(
        &self,
        ticket: &str,
        password: Option<&str>,
        seconds: u32,
    ) -> Result<ConnectionReport, NetError>;
    fn leave(&self);
    fn set_session_options(&self, _options: SessionOptions) {}
    fn set_player_settings(&self, _settings: PlayerSettings) {}
    fn status(&self) -> NetStatus;
}

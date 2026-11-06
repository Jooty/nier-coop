use crate::net::{ConnectionReport, LauncherNet, NetError, NetStatus};

pub struct StubNet;

impl LauncherNet for StubNet {
    fn host(&self, _password: Option<&str>) -> Result<String, NetError> {
        Err(NetError::Unavailable)
    }

    fn join(&self, _ticket: &str, _password: Option<&str>) -> Result<(), NetError> {
        Err(NetError::Unavailable)
    }

    fn test_connection(
        &self,
        _ticket: &str,
        _password: Option<&str>,
        _seconds: u32,
    ) -> Result<ConnectionReport, NetError> {
        Err(NetError::Unavailable)
    }

    fn leave(&self) {}

    fn status(&self) -> NetStatus {
        NetStatus::default()
    }
}

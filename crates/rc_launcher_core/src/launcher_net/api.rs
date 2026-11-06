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
    fn status(&self) -> NetStatus;
}

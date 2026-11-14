use nt_wire::RejectReason;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetState {
    #[default]
    WaitingForLauncher,
    LauncherLinked,
    Connected,
    Rejected(RejectReason),
    Failed(String),
}

impl NetState {
    pub fn label(&self) -> String {
        match self {
            Self::WaitingForLauncher => "waiting for the launcher".to_owned(),
            Self::LauncherLinked => "launcher linked, no other player".to_owned(),
            Self::Connected => "connected".to_owned(),
            Self::Rejected(reason) => format!("rejected ({reason:?})"),
            Self::Failed(message) => format!("failed ({message})"),
        }
    }
}

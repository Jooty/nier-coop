#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveRole {
    Host,
    Guest,
}

impl SaveRole {
    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Guest => "guest",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveRedirectStatus {
    NotRedirected,
    CoopFolder,
}

impl SaveRedirectStatus {
    pub fn status_line(self) -> &'static str {
        match self {
            Self::NotRedirected => "saves: NOT redirected, the normal save folder is in use",
            Self::CoopFolder => "saves: co-op folder",
        }
    }
}

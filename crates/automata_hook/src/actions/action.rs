use nc_hook::diagnostics::log_line;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    LogLine(String),
}

impl Action {
    pub(super) fn run(self) {
        match self {
            Self::LogLine(text) => log_line(&text),
        }
    }
}

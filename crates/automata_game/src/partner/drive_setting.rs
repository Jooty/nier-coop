#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerDrive {
    State,
    Input,
}

impl PartnerDrive {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "state" => Some(Self::State),
            "input" => Some(Self::Input),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Input => "input",
        }
    }
}

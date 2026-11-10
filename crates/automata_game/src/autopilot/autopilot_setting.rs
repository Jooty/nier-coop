/// A test aid: makes the local hero move with nobody at the PC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Autopilot {
    #[default]
    Off,
    Circle,
}

impl Autopilot {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "off" => Some(Self::Off),
            "circle" => Some(Self::Circle),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Circle => "circle",
        }
    }
}

/// The `story_steps` setting: `watch` only observes, `on` holds and applies, `off` hooks nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoryStepsMode {
    On,
    #[default]
    Watch,
    Off,
}

impl StoryStepsMode {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "on" => Some(Self::On),
            "watch" => Some(Self::Watch),
            "off" => Some(Self::Off),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Watch => "watch",
            Self::Off => "off",
        }
    }

    pub fn installs_hooks(self) -> bool {
        self != Self::Off
    }

    pub fn holds_and_applies(self) -> bool {
        self == Self::On
    }
}

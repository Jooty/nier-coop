#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnemyShare {
    #[default]
    On,
    Watch,
    Off,
}

impl EnemyShare {
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

    pub fn lists_enemies(self) -> bool {
        self != Self::Off
    }

    pub fn shares_health(self) -> bool {
        self == Self::On
    }

    pub fn lets_the_host_own_the_spawns(self) -> bool {
        self == Self::On
    }
}

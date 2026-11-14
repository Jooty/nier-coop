/// Where a class of story action is sorted, and so whether its done is taken from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepSort {
    Story,
    Area,
    OnePlayer,
    Presentation,
    LedByPhaseFollow,
    LedByEventFollow,
    LedByGeneratorCounts,
}

impl StepSort {
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "story" => Some(Self::Story),
            "area" => Some(Self::Area),
            "one_player" => Some(Self::OnePlayer),
            "presentation" => Some(Self::Presentation),
            "led_by_phase_follow" => Some(Self::LedByPhaseFollow),
            "led_by_event_follow" => Some(Self::LedByEventFollow),
            "led_by_generator_counts" => Some(Self::LedByGeneratorCounts),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Story => "story",
            Self::Area => "area",
            Self::OnePlayer => "one_player",
            Self::Presentation => "presentation",
            Self::LedByPhaseFollow => "led_by_phase_follow",
            Self::LedByEventFollow => "led_by_event_follow",
            Self::LedByGeneratorCounts => "led_by_generator_counts",
        }
    }

    pub fn is_mirrored(self) -> bool {
        matches!(self, Self::Story | Self::Area)
    }
}

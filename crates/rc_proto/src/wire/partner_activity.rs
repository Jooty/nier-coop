use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PartnerActivity {
    #[default]
    Playing,
    InMenu,
    InCutscene,
}

impl PartnerActivity {
    pub fn world_paused(self) -> bool {
        self != Self::Playing
    }

    pub fn label(self) -> Option<&'static str> {
        match self {
            Self::Playing => None,
            Self::InMenu => Some("in menu"),
            Self::InCutscene => Some("in a cutscene"),
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExeVariant {
    Main,
    Compat,
}

impl ExeVariant {
    pub const ALL: [Self; 2] = [Self::Main, Self::Compat];

    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Compat => "compat",
        }
    }
}

pub fn exe_difference_line(local: ExeVariant, partner: ExeVariant) -> Option<String> {
    (local != partner).then(|| {
        format!(
            "net: this game is the {} exe and the other player's is the {} exe; the two may play together",
            local.label(),
            partner.label()
        )
    })
}

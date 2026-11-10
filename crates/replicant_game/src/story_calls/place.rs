use crate::story_calls::SAME_PLACE_RANGE_UNITS;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Whereabouts {
    pub phase: u16,
    pub position: [f32; 3],
}

/// Where the partner stands as seen from this game, for scenes both players share.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PartnerPlace {
    Same,
    OtherPhase,
    TooFar { units: f32 },
    Unknown,
}

impl PartnerPlace {
    pub fn judge(local: Option<Whereabouts>, partner: Option<Whereabouts>) -> Self {
        let (Some(local), Some(partner)) = (local, partner) else {
            return Self::Unknown;
        };
        if local.phase != partner.phase {
            return Self::OtherPhase;
        }
        let units = local
            .position
            .iter()
            .zip(partner.position)
            .map(|(here, there)| (here - there) * (here - there))
            .sum::<f32>()
            .sqrt();
        if units.is_finite() && units <= SAME_PLACE_RANGE_UNITS {
            Self::Same
        } else {
            Self::TooFar { units }
        }
    }

    pub fn is_same(self) -> bool {
        self == Self::Same
    }

    pub fn describe(self) -> String {
        match self {
            Self::Same => "in the same place".to_owned(),
            Self::OtherPhase => "in another phase".to_owned(),
            Self::TooFar { units } => format!("{units:.1} units away"),
            Self::Unknown => "not known".to_owned(),
        }
    }
}

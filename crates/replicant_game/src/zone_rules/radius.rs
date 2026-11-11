use crate::zone_rules::{DEFAULT_ZONE_RADIUS_UNITS, MAX_ZONE_RADIUS_UNITS, MIN_ZONE_RADIUS_UNITS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneRadius {
    units: f32,
}

impl ZoneRadius {
    pub fn new(units: f32) -> Self {
        if units.is_finite() {
            Self {
                units: units.clamp(MIN_ZONE_RADIUS_UNITS, MAX_ZONE_RADIUS_UNITS),
            }
        } else {
            Self::default()
        }
    }

    pub fn units(self) -> f32 {
        self.units
    }
}

impl Default for ZoneRadius {
    fn default() -> Self {
        Self {
            units: DEFAULT_ZONE_RADIUS_UNITS,
        }
    }
}

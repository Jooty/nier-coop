use crate::stay_together::{DEFAULT_LEASH_UNITS, MAX_LEASH_UNITS, MIN_LEASH_UNITS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leash {
    units: f32,
}

impl Leash {
    pub fn new(units: f32) -> Self {
        if units.is_finite() {
            Self {
                units: units.clamp(MIN_LEASH_UNITS, MAX_LEASH_UNITS),
            }
        } else {
            Self::default()
        }
    }

    pub fn units(self) -> f32 {
        self.units
    }

    pub fn is_exceeded_by(self, distance: Option<f32>) -> bool {
        distance.is_some_and(|distance| distance > self.units)
    }
}

impl Default for Leash {
    fn default() -> Self {
        Self {
            units: DEFAULT_LEASH_UNITS,
        }
    }
}

pub fn distance_between(from: [f32; 3], to: [f32; 3]) -> Option<f32> {
    let squared: f32 = from.iter().zip(to).map(|(a, b)| (a - b) * (a - b)).sum();
    squared.is_finite().then(|| squared.sqrt())
}

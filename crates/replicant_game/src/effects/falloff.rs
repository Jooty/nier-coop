use crate::effects::{SOUND_FALLOFF_FAR_UNITS, SOUND_FALLOFF_NEAR_UNITS};

/// Volume factor for a sound `distance` units from the listener: full inside the near
/// distance, falling smoothly to silence at the far distance.
pub fn sound_gain(distance: f32) -> f32 {
    let span = SOUND_FALLOFF_FAR_UNITS - SOUND_FALLOFF_NEAR_UNITS;
    let travelled = ((distance - SOUND_FALLOFF_NEAR_UNITS) / span).clamp(0.0, 1.0);
    (1.0 - travelled).powi(2)
}

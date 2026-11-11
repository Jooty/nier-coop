use crate::offsets::{NOCLIP_FAST_MULTIPLIER, NOCLIP_UNITS_PER_STEP};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Displacement {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoclipInput {
    pub forward: i8,
    pub strafe: i8,
    pub vertical: i8,
    pub fast: bool,
}

impl NoclipInput {
    pub fn displacement(self) -> Displacement {
        let forward = f32::from(self.forward.clamp(-1, 1));
        let strafe = f32::from(self.strafe.clamp(-1, 1));
        let vertical = f32::from(self.vertical.clamp(-1, 1));
        let ground_length = forward.hypot(strafe).max(1.0);
        let speed = if self.fast {
            NOCLIP_UNITS_PER_STEP * NOCLIP_FAST_MULTIPLIER
        } else {
            NOCLIP_UNITS_PER_STEP
        };
        Displacement {
            x: strafe / ground_length * speed,
            y: vertical * speed,
            z: forward / ground_length * speed,
        }
    }
}

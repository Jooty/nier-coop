use core::f32::consts::TAU;

use crate::Position;
use crate::stay_together::{
    MAX_FLOOR_STEP_FROM_PARTNER, MIN_FLOOR_NORMAL_Y, SPOT_ANGLE_STEPS, SPOT_RING_RADII,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorHit {
    pub y: f32,
    pub normal_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotRefusal {
    NoGroundUnderPartner,
    NoSafeSpot,
}

pub trait WorldProbe {
    fn floor_below(&self, x: f32, y: f32, z: f32) -> Option<FloorHit>;

    fn body_fits(&self, x: f32, y: f32, z: f32) -> bool;
}

impl SpotRefusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoGroundUnderPartner => "there is no solid ground under the other player",
            Self::NoSafeSpot => "no safe spot next to the other player (wall, drop or slope)",
        }
    }
}

pub fn choose_spot(
    partner: Position,
    local: Position,
    probe: &impl WorldProbe,
) -> Result<Position, SpotRefusal> {
    let ground = probe
        .floor_below(partner.x, partner.y, partner.z)
        .ok_or(SpotRefusal::NoGroundUnderPartner)?;
    let bearing = (local.z - partner.z).atan2(local.x - partner.x);
    for radius in SPOT_RING_RADII {
        for step in nearest_first_steps() {
            let angle = bearing + TAU * step as f32 / SPOT_ANGLE_STEPS as f32;
            let x = partner.x + radius * angle.cos();
            let z = partner.z + radius * angle.sin();
            if let Some(spot) = standable_spot(probe, x, ground.y, z) {
                return Ok(spot);
            }
        }
    }
    Err(SpotRefusal::NoSafeSpot)
}

fn nearest_first_steps() -> Vec<u32> {
    let mut steps: Vec<u32> = (0..SPOT_ANGLE_STEPS).collect();
    steps.sort_by_key(|step| (*step).min(SPOT_ANGLE_STEPS - step));
    steps
}

fn standable_spot(probe: &impl WorldProbe, x: f32, ground_y: f32, z: f32) -> Option<Position> {
    let floor = probe.floor_below(x, ground_y, z)?;
    let flat_enough = floor.normal_y >= MIN_FLOOR_NORMAL_Y;
    let same_level = (floor.y - ground_y).abs() <= MAX_FLOOR_STEP_FROM_PARTNER;
    (flat_enough && same_level && probe.body_fits(x, floor.y, z)).then_some(Position {
        x,
        y: floor.y,
        z,
    })
}

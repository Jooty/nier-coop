use automata_game::partner::BodySnapshot;
use serde::{Deserialize, Serialize};

use crate::wire::{MAX_FACING_ABS_RADIANS, MAX_POSITION_ABS, MAX_SPEED_ABS, WireError};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub sequence: u32,
    pub held: u32,
    pub pressed: u32,
    pub button_mask: u8,
    pub speed: f32,
    pub world_facing: f32,
    pub position: [f32; 3],
    pub body_facing: f32,
    pub state: u16,
    pub variant: u32,
    pub b: u32,
    pub c: u32,
    pub health: i32,
}

impl PlayerState {
    pub fn from_snapshot(snapshot: &BodySnapshot, sequence: u32) -> Self {
        Self {
            sequence,
            held: snapshot.held,
            pressed: snapshot.pressed,
            button_mask: snapshot.button_mask,
            speed: snapshot.speed,
            world_facing: snapshot.world_facing,
            position: snapshot.position,
            body_facing: snapshot.body_facing,
            state: snapshot.state,
            variant: snapshot.variant,
            b: snapshot.b,
            c: snapshot.c,
            health: snapshot.health,
        }
    }

    pub fn to_snapshot(&self) -> BodySnapshot {
        BodySnapshot {
            held: self.held,
            pressed: self.pressed,
            button_mask: self.button_mask,
            speed: self.speed,
            world_facing: self.world_facing,
            position: self.position,
            body_facing: self.body_facing,
            state: self.state,
            variant: self.variant,
            b: self.b,
            c: self.c,
            health: self.health,
        }
    }

    pub fn validate(&self) -> Result<(), WireError> {
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange { field: "position" });
        }
        if !is_facing(self.world_facing) {
            return Err(WireError::OutOfRange {
                field: "world_facing",
            });
        }
        if !is_facing(self.body_facing) {
            return Err(WireError::OutOfRange {
                field: "body_facing",
            });
        }
        if !self.speed.is_finite() || self.speed.abs() > MAX_SPEED_ABS {
            return Err(WireError::OutOfRange { field: "speed" });
        }
        Ok(())
    }
}

fn is_facing(radians: f32) -> bool {
    radians.is_finite() && radians.abs() <= MAX_FACING_ABS_RADIANS
}

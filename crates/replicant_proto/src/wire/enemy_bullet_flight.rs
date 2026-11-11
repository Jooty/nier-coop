use serde::{Deserialize, Serialize};

use crate::wire::{BulletFlight, EnemyId, WireError};

pub const MAX_BULLET_SPRITE_KIND: u8 = 1;
pub const MAX_BULLET_SPRITE_SIZE: f32 = 1000.0;
pub const MAX_BULLET_SPRITE_ALPHA: f32 = 16.0;
pub const MAX_BULLET_SPRITE_ANGLE_ABS: f32 = 1000.0;

/// What the game's bullet sprite batch needs, besides a place, to draw one sprite-drawn bullet.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BulletSprite {
    pub kind: u8,
    pub size: f32,
    pub alpha: f32,
    /// Radians.
    pub angle: f32,
}

/// One step of a bullet fired by one of the host's enemies, for the guest to draw. The guest's
/// copy never fires; damage from the bullet reaches the guest as a `PlayerHit` from the host.
/// A sprite-drawn bullet carries `sprite` on every step and never a look.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemyBulletFlight {
    pub enemy: EnemyId,
    pub flight: BulletFlight,
    pub sprite: Option<BulletSprite>,
}

impl BulletSprite {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.kind > MAX_BULLET_SPRITE_KIND {
            return Err(WireError::OutOfRange {
                field: "bullet sprite kind",
            });
        }
        if !(0.0..=MAX_BULLET_SPRITE_SIZE).contains(&self.size) {
            return Err(WireError::OutOfRange {
                field: "bullet sprite size",
            });
        }
        if !(0.0..=MAX_BULLET_SPRITE_ALPHA).contains(&self.alpha) {
            return Err(WireError::OutOfRange {
                field: "bullet sprite alpha",
            });
        }
        if !self.angle.is_finite() || self.angle.abs() > MAX_BULLET_SPRITE_ANGLE_ABS {
            return Err(WireError::OutOfRange {
                field: "bullet sprite angle",
            });
        }
        Ok(())
    }
}

impl EnemyBulletFlight {
    pub fn validate(&self) -> Result<(), WireError> {
        self.enemy.validate()?;
        self.flight.validate()?;
        match &self.sprite {
            Some(sprite) => sprite.validate(),
            None => Ok(()),
        }
    }
}

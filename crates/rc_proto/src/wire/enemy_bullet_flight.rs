use serde::{Deserialize, Serialize};

use crate::wire::{BulletFlight, EnemyId, WireError};

/// One step of a bullet fired by one of the host's enemies, for the guest to draw. The guest's
/// copy never fires; damage from the bullet reaches the guest as a `PlayerHit` from the host.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemyBulletFlight {
    pub enemy: EnemyId,
    pub flight: BulletFlight,
}

impl EnemyBulletFlight {
    pub fn validate(&self) -> Result<(), WireError> {
        self.enemy.validate()?;
        self.flight.validate()
    }
}

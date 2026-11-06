use serde::{Deserialize, Serialize};

use crate::wire::{
    ENEMY_GROUP_COUNT, MAX_ENEMIES_PER_SNAPSHOT, MAX_ENEMY_SNAPSHOT_PARTS, MAX_HP,
    MAX_POSITION_ABS, MAX_YAW_ABS_RADIANS, MotionState, WireError,
};

/// A network id for one enemy: its script group and the order it spawned in within that group
/// since the phase began. Never a handle or a spawn serial, which differ between machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnemyId {
    pub group: u16,
    pub ordinal: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemyState {
    pub id: EnemyId,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
    pub hp: u32,
    pub max_hp: u32,
}

/// One part of the host's enemy snapshot for `tick`. A snapshot too big for one message is split
/// into `parts` messages that share the tick; `total` is how many enemies the host sees in all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemySnapshot {
    pub tick: u32,
    pub phase: u16,
    pub part: u8,
    pub parts: u8,
    pub total: u16,
    pub enemies: Vec<EnemyState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnemySpawned {
    pub phase: u16,
    pub enemy: EnemyState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyGoneReason {
    Killed,
    Removed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyGone {
    pub phase: u16,
    pub id: EnemyId,
    pub reason: EnemyGoneReason,
}

impl EnemyId {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.group >= ENEMY_GROUP_COUNT {
            return Err(WireError::OutOfRange {
                field: "enemy_group",
            });
        }
        Ok(())
    }
}

impl EnemyState {
    pub fn validate(&self) -> Result<(), WireError> {
        self.id.validate()?;
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "enemy_position",
            });
        }
        if !self.yaw_radians.is_finite() || self.yaw_radians.abs() > MAX_YAW_ABS_RADIANS {
            return Err(WireError::OutOfRange { field: "enemy_yaw" });
        }
        if let Some(motion) = &self.motion {
            motion.validate()?;
        }
        if self.max_hp > MAX_HP || self.hp > self.max_hp {
            return Err(WireError::OutOfRange { field: "enemy_hp" });
        }
        Ok(())
    }
}

impl EnemySnapshot {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.parts == 0 || self.parts > MAX_ENEMY_SNAPSHOT_PARTS || self.part >= self.parts {
            return Err(WireError::OutOfRange {
                field: "snapshot_part",
            });
        }
        if self.enemies.len() > MAX_ENEMIES_PER_SNAPSHOT {
            return Err(WireError::OutOfRange {
                field: "snapshot_enemies",
            });
        }
        self.enemies.iter().try_for_each(EnemyState::validate)
    }
}

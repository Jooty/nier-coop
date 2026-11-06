use serde::{Deserialize, Serialize};

use crate::wire::{
    ENEMY_GROUP_COUNT, MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT, MAX_POSITION_ABS,
    MAX_YAW_ABS_RADIANS, MotionState, NPC_UNGROUPED, WireError,
};

/// A network id for one NPC: its script group ([`NPC_UNGROUPED`] for none), its actor class (the
/// model's row in the game's class table) and the order it spawned in among the NPCs of that group
/// and class since the phase began. Never a handle or a spawn serial, which differ between machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NpcId {
    pub group: u16,
    pub class: u16,
    pub ordinal: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NpcState {
    pub id: NpcId,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
    pub visible: bool,
}

/// One part of the host's NPC snapshot for `tick`. A snapshot too big for one message is split
/// into `parts` messages that share the tick; `total` is how many NPCs the host sees in all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NpcSnapshot {
    pub tick: u32,
    pub phase: u16,
    pub part: u8,
    pub parts: u8,
    pub total: u16,
    pub npcs: Vec<NpcState>,
}

impl NpcId {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.group >= ENEMY_GROUP_COUNT && self.group != NPC_UNGROUPED {
            return Err(WireError::OutOfRange { field: "npc_group" });
        }
        Ok(())
    }
}

impl NpcState {
    pub fn validate(&self) -> Result<(), WireError> {
        self.id.validate()?;
        if self
            .position
            .iter()
            .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
        {
            return Err(WireError::OutOfRange {
                field: "npc_position",
            });
        }
        if !self.yaw_radians.is_finite() || self.yaw_radians.abs() > MAX_YAW_ABS_RADIANS {
            return Err(WireError::OutOfRange { field: "npc_yaw" });
        }
        if let Some(motion) = &self.motion {
            motion.validate()?;
        }
        Ok(())
    }
}

impl NpcSnapshot {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.parts == 0 || self.parts > MAX_NPC_SNAPSHOT_PARTS || self.part >= self.parts {
            return Err(WireError::OutOfRange {
                field: "npc_snapshot_part",
            });
        }
        if self.npcs.len() > MAX_NPCS_PER_SNAPSHOT {
            return Err(WireError::OutOfRange {
                field: "npc_snapshot_npcs",
            });
        }
        self.npcs.iter().try_for_each(NpcState::validate)
    }
}

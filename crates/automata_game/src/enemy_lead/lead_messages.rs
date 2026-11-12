use serde::{Deserialize, Serialize};

use crate::enemies::EnemyId;
use crate::enemy_lead::{
    ENEMY_POSES_IN_ONE_LEAD_MESSAGE, HIGHEST_POSE_COORDINATE_ABS, HIGHEST_POSE_FACING_ABS_RADIANS,
    MOST_ENEMIES_LED,
};

/// Whom an enemy on the host aims at, in words both copies can map to a body of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LeadTarget {
    #[default]
    Nobody,
    HostPlayer,
    GuestPlayer,
    SomethingElse,
}

/// One shared enemy as the host's game holds it: place, facing, the four state ints at +0x670
/// and how many state starts the host has counted for it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnemyPose {
    pub id: EnemyId,
    pub position: [f32; 3],
    pub facing: f32,
    pub state: [i32; 4],
    pub starts: u32,
    pub target: LeadTarget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnemyLead {
    pub poses: Vec<EnemyPose>,
}

impl EnemyPose {
    pub fn is_valid(&self) -> bool {
        self.position
            .iter()
            .all(|axis| axis.is_finite() && axis.abs() <= HIGHEST_POSE_COORDINATE_ABS)
            && self.facing.is_finite()
            && self.facing.abs() <= HIGHEST_POSE_FACING_ABS_RADIANS
    }
}

impl EnemyLead {
    pub fn is_valid(&self) -> bool {
        self.poses.len() <= ENEMY_POSES_IN_ONE_LEAD_MESSAGE
            && self.poses.iter().all(EnemyPose::is_valid)
    }
}

/// The host's poses of one round, cut into messages that each fit the wire; poses past the
/// most led enemies are left out.
pub fn lead_parts(poses: &[EnemyPose]) -> Vec<EnemyLead> {
    poses
        .iter()
        .filter(|pose| pose.is_valid())
        .take(MOST_ENEMIES_LED)
        .copied()
        .collect::<Vec<_>>()
        .chunks(ENEMY_POSES_IN_ONE_LEAD_MESSAGE)
        .map(|chunk| EnemyLead {
            poses: chunk.to_vec(),
        })
        .collect()
}

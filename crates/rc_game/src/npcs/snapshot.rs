use std::f32::consts::{PI, TAU};

use rc_proto::{
    MAX_MOTION_FRAME, MAX_MOTION_SPEED_ABS, MAX_NPC_SNAPSHOT_PARTS, MAX_NPCS_PER_SNAPSHOT,
    MAX_POSITION_ABS, MotionState, NpcId, NpcSnapshot, NpcState,
};

use crate::npcs::NpcSighting;
use crate::puppet::LayerMotion;

/// The wire form of one sighting, or `None` when its position is not a usable number.
pub fn npc_state(id: NpcId, sighting: &NpcSighting) -> Option<NpcState> {
    if sighting
        .position
        .iter()
        .any(|axis| !axis.is_finite() || axis.abs() > MAX_POSITION_ABS)
    {
        return None;
    }
    let yaw_radians = if sighting.yaw_radians.is_finite() {
        (sighting.yaw_radians + PI).rem_euclid(TAU) - PI
    } else {
        0.0
    };
    Some(NpcState {
        id,
        position: sighting.position,
        yaw_radians,
        motion: sighting.motion.and_then(motion_state),
        visible: sighting.visible,
    })
}

/// Splits one snapshot into message-sized parts. When there are more NPCs than the parts can
/// hold, the ones nearest to any player go first and the farthest are left out.
pub fn split_npc_snapshot(
    tick: u32,
    phase: u16,
    mut npcs: Vec<NpcState>,
    players: &[[f32; 3]],
) -> Vec<NpcSnapshot> {
    let total = u16::try_from(npcs.len()).unwrap_or(u16::MAX);
    npcs.sort_by(|a, b| {
        nearest_player_distance(a.position, players)
            .total_cmp(&nearest_player_distance(b.position, players))
            .then(a.id.cmp(&b.id))
    });
    npcs.truncate(MAX_NPCS_PER_SNAPSHOT * usize::from(MAX_NPC_SNAPSHOT_PARTS));
    let chunks: Vec<Vec<NpcState>> = if npcs.is_empty() {
        vec![Vec::new()]
    } else {
        npcs.chunks(MAX_NPCS_PER_SNAPSHOT)
            .map(<[NpcState]>::to_vec)
            .collect()
    };
    let parts = u8::try_from(chunks.len()).unwrap_or(MAX_NPC_SNAPSHOT_PARTS);
    (0_u8..)
        .zip(chunks)
        .map(|(part, npcs)| NpcSnapshot {
            tick,
            phase,
            part,
            parts,
            total,
            npcs,
        })
        .collect()
}

fn nearest_player_distance(position: [f32; 3], players: &[[f32; 3]]) -> f32 {
    players
        .iter()
        .map(|player| {
            position
                .iter()
                .zip(player)
                .map(|(a, b)| (a - b) * (a - b))
                .sum::<f32>()
        })
        .filter(|squared| squared.is_finite())
        .fold(f32::INFINITY, f32::min)
}

fn motion_state(motion: LayerMotion) -> Option<MotionState> {
    if !motion.frame.is_finite() || !motion.speed.is_finite() {
        return None;
    }
    Some(MotionState {
        index: u16::try_from(motion.index).ok()?,
        frame: motion.frame.clamp(0.0, MAX_MOTION_FRAME),
        speed: motion
            .speed
            .clamp(-MAX_MOTION_SPEED_ABS, MAX_MOTION_SPEED_ABS),
        looping: motion.looping,
    })
}

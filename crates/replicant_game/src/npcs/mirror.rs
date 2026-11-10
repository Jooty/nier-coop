use std::collections::BTreeMap;

use replicant_proto::{
    MotionState, NpcId, NpcSnapshot, NpcState, PartnerActivity, PlayerState, StateTrack,
};

use crate::npcs::NPC_MIRROR_STALE_MS;

/// The guest's picture of the host's NPCs, built from snapshots, with each NPC's movement
/// buffered so it can be replayed a little behind and smoothly, like the partner's.
#[derive(Debug, Default)]
pub struct NpcMirror {
    phase: Option<u16>,
    host_total: Option<u16>,
    last_heard_ms: Option<u64>,
    npcs: BTreeMap<NpcId, MirroredTrack>,
}

#[derive(Debug)]
struct MirroredTrack {
    track: StateTrack,
    latest: NpcState,
    seen_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MirroredNpc {
    pub id: NpcId,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub motion: Option<MotionState>,
    pub visible: bool,
}

impl NpcMirror {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn phase(&self) -> Option<u16> {
        self.phase
    }

    pub fn host_total(&self) -> Option<u16> {
        self.host_total
    }

    pub fn len(&self) -> usize {
        self.npcs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.npcs.is_empty()
    }

    /// Whether a snapshot arrived within the stale time, so a missing NPC really is missing.
    pub fn is_current(&self, now_ms: u64) -> bool {
        self.last_heard_ms
            .is_some_and(|heard| now_ms.saturating_sub(heard) <= NPC_MIRROR_STALE_MS)
    }

    pub fn receive_snapshot(&mut self, at_ms: u64, snapshot: &NpcSnapshot) {
        if self.phase != Some(snapshot.phase) {
            self.clear();
            self.phase = Some(snapshot.phase);
        }
        self.host_total = Some(snapshot.total);
        self.last_heard_ms = Some(at_ms);
        for npc in &snapshot.npcs {
            self.update(at_ms, snapshot.tick, snapshot.phase, *npc);
        }
    }

    pub fn prune(&mut self, now_ms: u64) {
        self.npcs
            .retain(|_, npc| now_ms.saturating_sub(npc.seen_at_ms) <= NPC_MIRROR_STALE_MS);
    }

    pub fn sample(&self, now_ms: u64, delay_ms: u64) -> Vec<MirroredNpc> {
        self.npcs
            .iter()
            .filter_map(|(id, npc)| {
                let sample = npc.track.sample(now_ms, delay_ms)?;
                Some(MirroredNpc {
                    id: *id,
                    position: sample.position,
                    yaw_radians: sample.yaw_radians,
                    motion: sample.motion,
                    visible: npc.latest.visible,
                })
            })
            .collect()
    }

    fn update(&mut self, at_ms: u64, tick: u32, phase: u16, npc: NpcState) {
        let entry = self.npcs.entry(npc.id).or_insert_with(|| MirroredTrack {
            track: StateTrack::new(),
            latest: npc,
            seen_at_ms: at_ms,
        });
        let as_state = PlayerState {
            tick,
            phase,
            position: npc.position,
            yaw_radians: npc.yaw_radians,
            motion: npc.motion,
            hp: 0,
            max_hp: 0,
            weapon: None,
            activity: PartnerActivity::Playing,
            mp: 0,
            max_mp: 0,
            level: 0,
        };
        if entry.track.push(at_ms, as_state) {
            entry.latest = npc;
        }
        entry.seen_at_ms = at_ms;
    }
}

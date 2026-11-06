use std::collections::{BTreeMap, BTreeSet};

use rc_proto::NpcId;

use crate::npcs::{MirroredNpc, NPC_LOCAL_ONLY_GRACE_MS};

/// What the guest does with its own NPCs this tick: `drive` pairs a local NPC with the host's
/// state, `hide` are local NPCs the host has not had for the grace time, `waiting` are local NPCs
/// still inside it, `missing_here` are host NPCs this machine has no match for.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NpcPlan {
    pub drive: Vec<MirroredNpc>,
    pub hide: Vec<NpcId>,
    pub waiting: Vec<NpcId>,
    pub missing_here: Vec<NpcId>,
    pub newly_hidden: Vec<NpcId>,
    pub newly_missing: Vec<NpcId>,
}

/// Matches the guest's NPCs with the host's by network id, remembering how long each local NPC
/// has gone unmatched so one late snapshot never hides an NPC.
#[derive(Debug, Default)]
pub struct NpcPlanner {
    unmatched_since: BTreeMap<NpcId, u64>,
    hidden: BTreeSet<NpcId>,
    missing: BTreeSet<NpcId>,
}

impl NpcPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// `host_current` is false while no recent snapshot arrived; then nothing is hidden or
    /// reported missing, since the host's list is not known.
    pub fn plan(
        &mut self,
        now_ms: u64,
        local: &[NpcId],
        remote: &[MirroredNpc],
        host_current: bool,
    ) -> NpcPlan {
        let remote_by_id: BTreeMap<NpcId, MirroredNpc> =
            remote.iter().map(|npc| (npc.id, *npc)).collect();
        let local_ids: BTreeSet<NpcId> = local.iter().copied().collect();
        self.unmatched_since.retain(|id, _| local_ids.contains(id));
        let mut plan = NpcPlan::default();
        for id in &local_ids {
            if let Some(host) = remote_by_id.get(id) {
                self.unmatched_since.remove(id);
                plan.drive.push(*host);
                continue;
            }
            if !host_current {
                continue;
            }
            let since = *self.unmatched_since.entry(*id).or_insert(now_ms);
            if now_ms.saturating_sub(since) >= NPC_LOCAL_ONLY_GRACE_MS {
                plan.hide.push(*id);
            } else {
                plan.waiting.push(*id);
            }
        }
        if host_current {
            plan.missing_here = remote_by_id
                .keys()
                .filter(|id| !local_ids.contains(id))
                .copied()
                .collect();
        }
        let hidden: BTreeSet<NpcId> = plan.hide.iter().copied().collect();
        let missing: BTreeSet<NpcId> = plan.missing_here.iter().copied().collect();
        plan.newly_hidden = hidden.difference(&self.hidden).copied().collect();
        plan.newly_missing = missing.difference(&self.missing).copied().collect();
        self.hidden = hidden;
        self.missing = missing;
        plan
    }
}

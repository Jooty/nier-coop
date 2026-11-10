use std::collections::{HashMap, HashSet};

use replicant_proto::EnemyId;

use crate::enemies::HOST_SPAWN_COPY_GRACE_MS;
use crate::offsets::{ENEMY_SPEC_SLOT_ARRAY_RVA, ENEMY_SPEC_SLOT_BYTES, ENEMY_SPEC_SLOT_COUNT};

#[derive(Debug, Default)]
pub struct CreationWatch {
    first_missing_ms: HashMap<EnemyId, u64>,
}

impl CreationWatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn due(&mut self, now_ms: u64, missing: &[EnemyId]) -> Vec<EnemyId> {
        self.first_missing_ms.retain(|id, _| missing.contains(id));
        let mut due = Vec::new();
        for id in missing {
            let first = *self.first_missing_ms.entry(*id).or_insert(now_ms);
            if now_ms.saturating_sub(first) >= HOST_SPAWN_COPY_GRACE_MS {
                due.push(*id);
            }
        }
        for id in &due {
            self.first_missing_ms.remove(id);
        }
        due
    }
}

pub fn missing_host_spawns(
    host_live: &[EnemyId],
    own: &HashSet<EnemyId>,
    already_created: &HashSet<EnemyId>,
    ended: &HashSet<EnemyId>,
    creation_known: &HashSet<EnemyId>,
) -> Vec<EnemyId> {
    let mut missing: Vec<EnemyId> = host_live
        .iter()
        .filter(|id| {
            !own.contains(id)
                && !already_created.contains(id)
                && !ended.contains(id)
                && creation_known.contains(id)
        })
        .copied()
        .collect();
    missing.sort();
    missing
}

pub fn spec_slot_address(module_base: u64, slot: u16) -> Option<u64> {
    (usize::from(slot) < ENEMY_SPEC_SLOT_COUNT).then(|| {
        module_base
            + u64::from(ENEMY_SPEC_SLOT_ARRAY_RVA)
            + u64::from(slot) * ENEMY_SPEC_SLOT_BYTES as u64
    })
}

pub fn spec_slot_index(module_base: u64, slot_address: u64) -> Option<u16> {
    let offset = slot_address.checked_sub(module_base + u64::from(ENEMY_SPEC_SLOT_ARRAY_RVA))?;
    let slot_bytes = ENEMY_SPEC_SLOT_BYTES as u64;
    if offset % slot_bytes != 0 {
        return None;
    }
    let index = usize::try_from(offset / slot_bytes).ok()?;
    if index >= ENEMY_SPEC_SLOT_COUNT {
        return None;
    }
    u16::try_from(index).ok()
}

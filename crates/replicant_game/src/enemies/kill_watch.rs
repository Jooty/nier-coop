use std::collections::{BTreeMap, BTreeSet};

use replicant_proto::EnemyId;

/// Spots each enemy death on the host once: when an enemy seen alive is next seen at zero HP, or
/// leaves as killed without that being seen.
#[derive(Debug, Default)]
pub struct KillWatch {
    phase: Option<u16>,
    last_hp: BTreeMap<EnemyId, i32>,
    reported: BTreeSet<EnemyId>,
}

impl KillWatch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// `seen` is every enemy of this scan with its HP; `killed_and_gone` the ones that left as
    /// killed. Returns the deaths not reported before, in id order.
    pub fn observe(
        &mut self,
        phase: u16,
        seen: &[(EnemyId, i32)],
        killed_and_gone: &[EnemyId],
    ) -> Vec<EnemyId> {
        if self.phase != Some(phase) {
            self.clear();
            self.phase = Some(phase);
        }
        let mut deaths = BTreeSet::new();
        for (id, hp) in seen {
            let was_alive = self.last_hp.get(id).is_some_and(|last| *last > 0);
            if *hp <= 0 && was_alive && !self.reported.contains(id) {
                deaths.insert(*id);
            }
            if *hp > 0 {
                self.reported.remove(id);
            }
            self.last_hp.insert(*id, *hp);
        }
        for id in killed_and_gone {
            if !self.reported.contains(id) {
                deaths.insert(*id);
            }
            self.last_hp.remove(id);
        }
        self.reported.extend(deaths.iter().copied());
        for id in killed_and_gone {
            self.reported.remove(id);
        }
        deaths.into_iter().collect()
    }
}

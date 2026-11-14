use std::collections::HashMap;

use automata_game::enemies::{
    EnemyHereForADefeat, EnemyId, UNFORCED_DEFEAT_IS_EXPLAINED_AFTER_MS, unforced_defeat_line,
    unforced_defeat_reason,
};

use crate::enemies_driver::driver_state::EnemiesDriver;
use crate::game_calls::EnemyReading;

#[derive(Debug, Clone, Copy)]
pub struct HeardDefeatWait {
    heard_at_ms: u64,
    explained: bool,
}

impl EnemiesDriver {
    pub(super) fn wait_to_see_the_defeat_forced(&mut self, id: EnemyId, now_ms: u64) {
        self.heard_defeats_not_forced
            .entry(id)
            .or_insert(HeardDefeatWait {
                heard_at_ms: now_ms,
                explained: false,
            });
    }

    pub(super) fn explain_the_unforced_defeats(
        &mut self,
        now_ms: u64,
        readings: &[EnemyReading],
        doubled: &[EnemyId],
    ) {
        let theirs: HashMap<EnemyId, u32> = self
            .other_roster
            .iter()
            .flatten()
            .map(|sighting| (sighting.id, sighting.object_id))
            .collect();
        let waiting: Vec<(EnemyId, HeardDefeatWait)> = self
            .heard_defeats_not_forced
            .iter()
            .map(|(id, wait)| (*id, *wait))
            .collect();
        for (id, wait) in waiting {
            let here = readings.iter().find(|reading| reading.sighting.id == id);
            let forced =
                here.is_some_and(|reading| self.handles_defeated_here.contains(&reading.handle));
            if !self.defeats.holds(id) || forced {
                self.heard_defeats_not_forced.remove(&id);
                continue;
            }
            let heard_ago_ms = now_ms.saturating_sub(wait.heard_at_ms);
            if wait.explained || heard_ago_ms < UNFORCED_DEFEAT_IS_EXPLAINED_AFTER_MS {
                continue;
            }
            let reason = unforced_defeat_reason(
                here.map(|reading| EnemyHereForADefeat {
                    sighting: reading.sighting,
                    uses_the_base_damage_code: reading.uses_the_base_damage_code,
                }),
                doubled.contains(&id),
                theirs.get(&id).copied(),
            );
            if let Some(reason) = reason {
                if let Some(wait) = self.heard_defeats_not_forced.get_mut(&id) {
                    wait.explained = true;
                }
                self.log_share_line(unforced_defeat_line(id, reason, heard_ago_ms));
            }
        }
    }
}

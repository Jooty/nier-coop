use std::collections::{HashMap, HashSet};

use rc_proto::EnemyId;

use crate::enemies::EnemySighting;

const MAX_TAKEOVER_DISTANCE: f32 = 3.0;
pub const FIGHT_SPAWN_FILL_IN_GRACE_MS: u64 = 3000;

pub fn plan_takeovers(
    own_spawns: &[(EnemyId, EnemySighting)],
    copies: &[(EnemyId, EnemySighting)],
    host_live: &[EnemyId],
    already_own: &HashSet<EnemyId>,
) -> Vec<(EnemyId, EnemyId)> {
    let mut pairs: Vec<(f32, EnemyId, EnemyId)> = Vec::new();
    for (own, own_sighting) in own_spawns {
        if host_live.contains(own) {
            continue;
        }
        for (copy, copy_sighting) in copies {
            if already_own.contains(copy) {
                continue;
            }
            if copy.group == own.group
                && copy_sighting.group == own_sighting.group
                && copy_sighting.class_id == own_sighting.class_id
                && copy_sighting.actor != own_sighting.actor
            {
                let gap = distance(own_sighting.position, copy_sighting.position);
                if gap <= MAX_TAKEOVER_DISTANCE {
                    pairs.push((gap, *own, *copy));
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut taken_own = HashSet::new();
    let mut taken_copy = HashSet::new();
    let mut plan = Vec::new();
    for (_, own, copy) in pairs {
        if taken_own.contains(&own) || taken_copy.contains(&copy) {
            continue;
        }
        taken_own.insert(own);
        taken_copy.insert(copy);
        plan.push((own, copy));
    }
    plan.sort();
    plan
}

/// What the guest's own spawns inside a fight window do with the host ids their order names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FightBindingPlan {
    /// (own id, host id): the own enemy takes the host id no enemy here holds.
    pub renames: Vec<(EnemyId, EnemyId)>,
    /// (own id, host id): a copy here holds the host id; the own enemy takes it over and the copy goes.
    pub takeovers: Vec<(EnemyId, EnemyId)>,
}

/// Inside a fight window the guest's own game spawns each wave from the host's replayed handler,
/// so its n-th own spawn of a group is the host's n-th: each own spawn binds to the host id
/// (group, own arrival). A copy made here from the host's spawn that holds that id is taken over
/// (so a filled-in copy never doubles the late own spawn); an id an own enemy holds here or an own
/// actor ever took over is never bound again.
pub fn plan_fight_bindings(
    own_spawns: &[(EnemyId, u16)],
    held_here: &HashSet<EnemyId>,
    copies_here: &HashSet<EnemyId>,
    already_own: &HashSet<EnemyId>,
) -> FightBindingPlan {
    let mut held = held_here.clone();
    let mut copies = copies_here.clone();
    let mut spawns = own_spawns.to_vec();
    spawns.sort();
    let mut plan = FightBindingPlan::default();
    for (own, arrival) in spawns {
        let bound = EnemyId {
            group: own.group,
            ordinal: arrival,
        };
        if bound == own || already_own.contains(&bound) {
            continue;
        }
        if copies.remove(&bound) {
            plan.takeovers.push((own, bound));
            continue;
        }
        if held.contains(&bound) {
            continue;
        }
        held.insert(bound);
        plan.renames.push((own, bound));
    }
    plan
}

/// Host enemies inside a fight window that the guest's replay has not spawned: each waits
/// `FIGHT_SPAWN_FILL_IN_GRACE_MS` from its arrival here for a guest spawn to bind it, then is due
/// for one copy.
#[derive(Debug, Clone, Default)]
pub struct FightSpawnFillIn {
    waiting_since: HashMap<EnemyId, u64>,
    filled_in: HashSet<EnemyId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FightFillInStep {
    /// Host ids that just arrived with no enemy here holding them.
    pub newly_waiting: Vec<EnemyId>,
    /// (host id, ms waited): no guest spawn bound it in time, so a copy goes in now.
    pub due: Vec<(EnemyId, u64)>,
}

impl FightSpawnFillIn {
    /// One guest tick inside a fight window: `host_live` are the host's enemies, `held_here` the
    /// ids an enemy here holds, `ended` the ids killed or gone, `copyable` the ids a copy can be
    /// made of (creation known, not a boss).
    pub fn observe(
        &mut self,
        now_ms: u64,
        host_live: &[EnemyId],
        held_here: &HashSet<EnemyId>,
        ended: &HashSet<EnemyId>,
        copyable: &HashSet<EnemyId>,
    ) -> FightFillInStep {
        let waiting: HashSet<EnemyId> = host_live
            .iter()
            .filter(|id| !held_here.contains(id) && !ended.contains(id))
            .copied()
            .collect();
        self.waiting_since.retain(|id, _| waiting.contains(id));
        let mut step = FightFillInStep::default();
        let mut ids: Vec<EnemyId> = waiting.into_iter().collect();
        ids.sort();
        for id in ids {
            if self.filled_in.contains(&id) {
                continue;
            }
            let since = *self.waiting_since.entry(id).or_insert_with(|| {
                step.newly_waiting.push(id);
                now_ms
            });
            let waited = now_ms.saturating_sub(since);
            if waited >= FIGHT_SPAWN_FILL_IN_GRACE_MS && copyable.contains(&id) {
                step.due.push((id, waited));
            }
        }
        step
    }

    pub fn mark_filled_in(&mut self, id: EnemyId) {
        self.waiting_since.remove(&id);
        self.filled_in.insert(id);
    }

    pub fn clear(&mut self) {
        self.waiting_since.clear();
        self.filled_in.clear();
    }
}

pub fn fight_spawn_filled_in_text(id: EnemyId, waited_ms: u64) -> String {
    format!(
        "fight spawn filled in {}:{} after {waited_ms}",
        id.group, id.ordinal
    )
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

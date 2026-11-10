use std::collections::HashSet;

use replicant_proto::EnemyId;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct GhostPlan {
    pub adopt: Vec<u64>,
    pub release: Vec<u64>,
}

pub fn plan_ghosts(
    ghosts: &[(u64, EnemyId)],
    host_live: &HashSet<EnemyId>,
    host_ended_groups: &HashSet<u16>,
) -> GhostPlan {
    let live_groups: HashSet<u16> = host_live.iter().map(|id| id.group).collect();
    let mut plan = GhostPlan::default();
    for (actor, id) in ghosts {
        if host_live.contains(id) {
            plan.adopt.push(*actor);
        } else if host_ended_groups.contains(&id.group) && !live_groups.contains(&id.group) {
            plan.release.push(*actor);
        }
    }
    plan
}

pub fn ghost_is_same_spawn(seen_serial: Option<i32>, ghost_serial: i32) -> bool {
    seen_serial == Some(ghost_serial)
}

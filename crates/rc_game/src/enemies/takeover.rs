use std::collections::HashSet;

use rc_proto::EnemyId;

use crate::enemies::EnemySighting;

const MAX_TAKEOVER_DISTANCE: f32 = 3.0;

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

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

use std::collections::BTreeMap;

use replicant_proto::EnemyId;

/// How the guest's own enemies line up with the host's by network id, and how far apart the
/// matched pairs stand (in game units).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnemyMatch {
    pub local_count: usize,
    pub remote_count: usize,
    pub matched: Vec<(EnemyId, f32)>,
    pub unmatched_local: Vec<EnemyId>,
    pub unmatched_remote: Vec<EnemyId>,
}

impl EnemyMatch {
    pub fn worst_error(&self) -> Option<(EnemyId, f32)> {
        self.matched
            .iter()
            .copied()
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

pub fn match_enemies(local: &[(EnemyId, [f32; 3])], remote: &[(EnemyId, [f32; 3])]) -> EnemyMatch {
    let local_by_id: BTreeMap<EnemyId, [f32; 3]> = local.iter().copied().collect();
    let remote_by_id: BTreeMap<EnemyId, [f32; 3]> = remote.iter().copied().collect();
    let mut report = EnemyMatch {
        local_count: local_by_id.len(),
        remote_count: remote_by_id.len(),
        ..EnemyMatch::default()
    };
    for (id, here) in &local_by_id {
        match remote_by_id.get(id) {
            Some(there) => report.matched.push((*id, distance(*here, *there))),
            None => report.unmatched_local.push(*id),
        }
    }
    report.unmatched_remote = remote_by_id
        .keys()
        .filter(|id| !local_by_id.contains_key(id))
        .copied()
        .collect();
    report
}

fn distance(from: [f32; 3], to: [f32; 3]) -> f32 {
    from.iter()
        .zip(to)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

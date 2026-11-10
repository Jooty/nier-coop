use std::collections::HashMap;

use replicant_proto::EnemyId;

use crate::enemies::EnemySyncRole;
use crate::enemies::combat_view::with_view;

/// This frame's enemy sync role and the local actors that are shared enemies (the host's real
/// enemies, or the guest's copies), by actor address. Read-only.
#[derive(Debug, Clone, Default)]
pub struct SharedEnemyActors {
    pub role: Option<EnemySyncRole>,
    pub actors: HashMap<u64, EnemyId>,
}

pub fn shared_enemy_actors() -> SharedEnemyActors {
    with_view(|view| SharedEnemyActors {
        role: view.role,
        actors: view.enemies.clone(),
    })
}

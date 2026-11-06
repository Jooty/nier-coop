use std::collections::{HashMap, HashSet};

use rc_game::GameMemory;
use rc_game::enemies::{AggroPlanner, GuestHitGuard, KillWatch};
use rc_game::offsets::{ACTOR_KIND_MASK_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET};
use rc_game::puppet::actor_is_live;
use rc_proto::{EnemyDrop, EnemyId, EnemyKilled, PlayerHit};

use crate::enemies::counters::CombatCounts;
use crate::memory::InProcessMemory;

/// Everything the shared combat remembers between ticks, on either side.
#[derive(Debug, Default)]
pub(super) struct CombatState {
    pub(super) actors: HashMap<EnemyId, LocalEnemy>,
    pub(super) aggro: AggroPlanner,
    pub(super) kills: KillWatch,
    pub(super) guard: GuestHitGuard,
    pub(super) guest_kills: HashSet<EnemyId>,
    pub(super) puppet_kind: Option<PuppetKind>,
    pub(super) killed_copies: HashSet<u64>,
    pub(super) pending_kills: Vec<(EnemyKilled, u64)>,
    pub(super) pending_player_hits: Vec<PlayerHit>,
    pub(super) pending_drops: Vec<EnemyDrop>,
    pub(super) last_started: HashMap<EnemyId, i32>,
    pub(super) counts: CombatCounts,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalEnemy {
    pub(super) actor: u64,
    pub(super) serial: i32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PuppetKind {
    pub(super) actor: u64,
    pub(super) original: i32,
}

impl CombatState {
    pub(super) fn reset(&mut self) {
        let counts = self.counts;
        *self = Self::default();
        self.counts = counts;
    }

    pub(super) fn remember(&mut self, seen: impl IntoIterator<Item = (EnemyId, u64, i32)>) {
        self.actors = seen
            .into_iter()
            .map(|(id, actor, serial)| (id, LocalEnemy { actor, serial }))
            .collect();
    }

    pub(super) fn actor_of(&self, memory: &InProcessMemory, id: EnemyId) -> Option<u64> {
        let enemy = self.actors.get(&id)?;
        let still_it = actor_is_live(memory, enemy.actor).unwrap_or(false)
            && memory
                .read_i32(enemy.actor + ACTOR_SPAWN_SERIAL_OFFSET)
                .is_ok_and(|serial| serial == enemy.serial);
        still_it.then_some(enemy.actor)
    }

    pub(super) fn actor_ids(&self) -> HashMap<u64, EnemyId> {
        self.actors
            .iter()
            .map(|(id, enemy)| (enemy.actor, *id))
            .collect()
    }
}

pub(super) fn read_kind(memory: &InProcessMemory, actor: u64) -> Option<i32> {
    memory.read_i32(actor + ACTOR_KIND_MASK_OFFSET).ok()
}

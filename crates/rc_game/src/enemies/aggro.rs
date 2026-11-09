use std::collections::BTreeMap;

use rc_proto::EnemyId;

use crate::enemies::{AGGRO_PUSH_RANGE, AGGRO_REFRESH_MS, AGGRO_SWITCH_MARGIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggroTarget {
    Host,
    Guest,
}

/// Where the two players stand for the host's aggro choice; a downed or absent player is `None`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AggroPlayers {
    pub host: Option<[f32; 3]>,
    pub guest: Option<[f32; 3]>,
}

/// The host's plan for which of its enemies to point at which player. The game's own targeting
/// already finds the host's player; this only pushes an enemy onto the guest's body when the
/// guest is the nearer living player, and back onto the host when that stops being true.
#[derive(Debug, Default)]
pub struct AggroPlanner {
    pushed: BTreeMap<EnemyId, Pushed>,
}

#[derive(Debug, Clone, Copy)]
struct Pushed {
    target: AggroTarget,
    at_ms: u64,
}

impl AggroPlanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.pushed.clear();
    }

    pub fn targets(&self) -> BTreeMap<EnemyId, AggroTarget> {
        self.pushed
            .iter()
            .map(|(id, pushed)| (*id, pushed.target))
            .collect()
    }

    pub fn pushed_to_guest(&self) -> usize {
        self.pushed
            .values()
            .filter(|pushed| pushed.target == AggroTarget::Guest)
            .count()
    }

    /// The pushes to make now, one per enemy that needs one.
    pub fn plan(
        &mut self,
        now_ms: u64,
        enemies: &[(EnemyId, [f32; 3])],
        players: AggroPlayers,
    ) -> Vec<(EnemyId, AggroTarget)> {
        self.pushed
            .retain(|id, _| enemies.iter().any(|(present, _)| present == id));
        let mut pushes = Vec::new();
        for (id, position) in enemies {
            let last = self.pushed.get(id).copied();
            let Some(target) = choose_target(*position, players, last.map(|pushed| pushed.target))
            else {
                if last.is_some_and(|pushed| pushed.target == AggroTarget::Guest)
                    && players.host.is_some()
                {
                    self.record(&mut pushes, now_ms, *id, AggroTarget::Host);
                } else {
                    self.pushed.remove(id);
                }
                continue;
            };
            let due = match (target, last) {
                (AggroTarget::Guest, Some(pushed)) if pushed.target == AggroTarget::Guest => {
                    now_ms.saturating_sub(pushed.at_ms) >= AGGRO_REFRESH_MS
                }
                (AggroTarget::Guest, _) => true,
                (AggroTarget::Host, Some(pushed)) => pushed.target == AggroTarget::Guest,
                (AggroTarget::Host, None) => false,
            };
            if due {
                self.record(&mut pushes, now_ms, *id, target);
            }
        }
        pushes
    }

    fn record(
        &mut self,
        pushes: &mut Vec<(EnemyId, AggroTarget)>,
        now_ms: u64,
        id: EnemyId,
        target: AggroTarget,
    ) {
        self.pushed.insert(
            id,
            Pushed {
                target,
                at_ms: now_ms,
            },
        );
        pushes.push((id, target));
    }
}

/// The nearer living player within reach of the enemy, keeping `current` unless the other
/// player is nearer by more than the switch margin.
pub fn choose_target(
    enemy: [f32; 3],
    players: AggroPlayers,
    current: Option<AggroTarget>,
) -> Option<AggroTarget> {
    let reach = |player: Option<[f32; 3]>| {
        player
            .map(|at| distance(enemy, at))
            .filter(|gap| *gap <= AGGRO_PUSH_RANGE)
    };
    match (reach(players.host), reach(players.guest)) {
        (None, None) => None,
        (Some(_), None) => Some(AggroTarget::Host),
        (None, Some(_)) => Some(AggroTarget::Guest),
        (Some(host), Some(guest)) => Some(match current {
            Some(AggroTarget::Guest) if guest <= host + AGGRO_SWITCH_MARGIN => AggroTarget::Guest,
            Some(AggroTarget::Host) if host <= guest + AGGRO_SWITCH_MARGIN => AggroTarget::Host,
            _ if guest < host => AggroTarget::Guest,
            _ => AggroTarget::Host,
        }),
    }
}

fn distance(from: [f32; 3], to: [f32; 3]) -> f32 {
    from.iter()
        .zip(to)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

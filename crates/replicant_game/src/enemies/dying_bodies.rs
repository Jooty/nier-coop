use std::collections::BTreeMap;

use replicant_proto::EnemyId;

/// The host's dead enemies whose bodies are still in its world, so the guest can drop its copy's
/// body when the host's goes. Keyed by id; the actor and spawn serial are this machine's.
#[derive(Debug, Default)]
pub struct DyingBodies {
    phase: Option<u16>,
    bodies: BTreeMap<EnemyId, DyingBody>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DyingBody {
    actor: u64,
    serial: i32,
    died_at_ms: u64,
}

impl DyingBodies {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }

    pub fn track(&mut self, phase: u16, id: EnemyId, actor: u64, serial: i32, died_at_ms: u64) {
        self.enter_phase(phase);
        self.bodies.entry(id).or_insert(DyingBody {
            actor,
            serial,
            died_at_ms,
        });
    }

    /// The bodies that left since the last call, with the time since their death, in id order.
    /// `still_there(actor, serial)` says whether that actor is still the same spawn.
    pub fn gone(
        &mut self,
        phase: u16,
        now_ms: u64,
        mut still_there: impl FnMut(u64, i32) -> bool,
    ) -> Vec<(EnemyId, u64)> {
        self.enter_phase(phase);
        let mut gone = Vec::new();
        self.bodies.retain(|id, body| {
            let there = still_there(body.actor, body.serial);
            if !there {
                gone.push((*id, now_ms.saturating_sub(body.died_at_ms)));
            }
            there
        });
        gone
    }

    fn enter_phase(&mut self, phase: u16) {
        if self.phase != Some(phase) {
            self.bodies.clear();
            self.phase = Some(phase);
        }
    }
}

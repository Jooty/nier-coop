use std::collections::{BTreeMap, BTreeSet};

use crate::enemies::{
    ENEMY_HITS_IN_ONE_MESSAGE, EnemyHit, EnemyHits, EnemyId, EnemySighting,
    HOST_HEALTH_ABOVE_THE_LOCAL_ONE_WAITS_AFTER_A_LOCAL_HIT_MS, MOST_DEFEATED_ENEMIES_REMEMBERED,
};
use crate::save_redirect::SaveRole;

/// Whose attack a hit on an enemy came from, on the copy that saw the hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitOwner {
    LocalPlayer,
    PartnerBody,
}

/// What a copy does with a hit its own game just applied to a shared enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitSettlement {
    pub health_written: Option<i32>,
    pub amount_reported: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestHealthStep {
    Leave,
    Write(i32),
    Defeat,
}

/// Health the guest's own hits took, summed per enemy until the next send.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HitLedger {
    pending: BTreeMap<EnemyId, EnemyHit>,
}

/// The enemies a copy holds as defeated: on the host the ones it has announced, on the guest the
/// ones it has heard of. An enemy seen alive again leaves the set.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DefeatWatch {
    defeated: BTreeSet<EnemyId>,
}

impl HitLedger {
    pub fn add(&mut self, id: EnemyId, amount: u32, max_health: i32) {
        let hit = self.pending.entry(id).or_insert(EnemyHit {
            id,
            amount: 0,
            max_health,
        });
        hit.amount = hit.amount.saturating_add(amount);
        hit.max_health = max_health;
    }

    pub fn take_messages(&mut self) -> Vec<EnemyHits> {
        let hits: Vec<EnemyHit> = core::mem::take(&mut self.pending).into_values().collect();
        hits.chunks(ENEMY_HITS_IN_ONE_MESSAGE)
            .map(|chunk| EnemyHits {
                hits: chunk.to_vec(),
            })
            .collect()
    }
}

impl DefeatWatch {
    /// The host's side: the enemies of this count that are newly at no health.
    pub fn newly_defeated(&mut self, sightings: &[EnemySighting]) -> Vec<EnemyId> {
        self.forget_the_ones_alive_in(sightings);
        let new: Vec<EnemyId> = sightings
            .iter()
            .filter(|sighting| sighting.is_defeated() && !self.defeated.contains(&sighting.id))
            .map(|sighting| sighting.id)
            .collect();
        for id in &new {
            self.hear(*id);
        }
        new
    }

    pub fn hear(&mut self, id: EnemyId) {
        if self.defeated.len() >= MOST_DEFEATED_ENEMIES_REMEMBERED {
            self.defeated.pop_first();
        }
        self.defeated.insert(id);
    }

    pub fn forget_the_ones_alive_in(&mut self, sightings: &[EnemySighting]) {
        for sighting in sightings.iter().filter(|sighting| !sighting.is_defeated()) {
            self.defeated.remove(&sighting.id);
        }
    }

    pub fn holds(&self, id: EnemyId) -> bool {
        self.defeated.contains(&id)
    }

    pub fn all(&self) -> Vec<EnemyId> {
        self.defeated.iter().copied().collect()
    }

    pub fn forget(&mut self) {
        self.defeated.clear();
    }
}

/// A hit whose attacker is the body the other player drives here belongs to that player's copy.
pub fn hit_owner(attacker_handle: Option<u32>, partner_body_handle: Option<u32>) -> HitOwner {
    match (attacker_handle, partner_body_handle) {
        (Some(attacker), Some(partner)) if attacker == partner => HitOwner::PartnerBody,
        _ => HitOwner::LocalPlayer,
    }
}

/// The partner body's hit is taken back on both copies (its own copy counts it). The host's own
/// hit stands. The guest's own hit is reported to the host and never takes the last health here.
pub fn settle_hit(role: SaveRole, owner: HitOwner, before: i32, after: i32) -> HitSettlement {
    if after >= before || before < 1 {
        return HitSettlement::default();
    }
    match (owner, role) {
        (HitOwner::PartnerBody, _) => HitSettlement {
            health_written: Some(before),
            amount_reported: 0,
        },
        (HitOwner::LocalPlayer, SaveRole::Host) => HitSettlement::default(),
        (HitOwner::LocalPlayer, SaveRole::Guest) => HitSettlement {
            health_written: (after < 1).then_some(1),
            amount_reported: before.saturating_sub(after.max(0)) as u32,
        },
    }
}

/// The guest's amount as the same share of the host's full health; a hit never rounds to nothing.
pub fn amount_in_the_hosts_scale(amount: u32, guest_max_health: i32, host_max_health: i32) -> u32 {
    if amount == 0 || guest_max_health < 1 || host_max_health < 1 {
        return amount;
    }
    let scaled = u64::from(amount) * host_max_health as u64 / guest_max_health as u64;
    scaled.clamp(1, u64::from(u32::MAX)) as u32
}

/// The host's health as the same share of the guest's full health: at least 1 while the host's
/// enemy lives, 0 when it does not.
pub fn health_in_the_guests_scale(
    host_health: i32,
    host_max_health: i32,
    guest_max_health: i32,
) -> i32 {
    if host_health < 1 {
        return 0;
    }
    if host_max_health < 1 || guest_max_health < 1 {
        return host_health;
    }
    let scaled = i64::from(host_health) * i64::from(guest_max_health) / i64::from(host_max_health);
    scaled.clamp(1, i64::from(guest_max_health)) as i32
}

/// What the guest does to its own enemy for the host's word on the enemy with the same id.
pub fn guest_health_step(
    local: &EnemySighting,
    host: Option<&EnemySighting>,
    defeat_heard: bool,
    ms_since_local_hit: Option<u64>,
) -> GuestHealthStep {
    if local.is_defeated() {
        return GuestHealthStep::Leave;
    }
    if host.is_some_and(|host| host.object_id != local.object_id) {
        return GuestHealthStep::Leave;
    }
    if defeat_heard || host.is_some_and(EnemySighting::is_defeated) {
        return GuestHealthStep::Defeat;
    }
    let Some(host) = host else {
        return GuestHealthStep::Leave;
    };
    let wanted = health_in_the_guests_scale(host.health, host.max_health, local.max_health);
    let own_hit_is_fresh = ms_since_local_hit
        .is_some_and(|ms| ms < HOST_HEALTH_ABOVE_THE_LOCAL_ONE_WAITS_AFTER_A_LOCAL_HIT_MS);
    if wanted == local.health || (wanted > local.health && own_hit_is_fresh) {
        GuestHealthStep::Leave
    } else {
        GuestHealthStep::Write(wanted)
    }
}

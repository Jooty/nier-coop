use rc_proto::AttackGaugeHit;

use crate::attack_gauge::{ATTACK_GAUGE_PERCENT_DIVISOR, GAUGE_HIT_SEQ_MEMORY_ENTRIES};

/// What the host knows when a guest gauge hit arrives: its phase, the number of the opening it
/// shows (`None` while closed), the hit node's hp and max hp (`None` when there is no such node)
/// and the game's hp floor percent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GaugeHitFacts {
    pub local_phase: u16,
    pub serial: Option<u32>,
    pub node_hp: Option<(i32, i32)>,
    pub floor_percent: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeHitDrop {
    OtherPhase,
    Closed,
    OtherOpening,
    NoSuchNode,
    NodeAlreadyBroken,
    Repeated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GaugeHitVerdict {
    Apply {
        hp_before: i32,
        hp_after: i32,
        breaks: bool,
    },
    Drop(GaugeHitDrop),
}

/// The host's ledger of guest gauge hits: every hit is applied once, by its number.
#[derive(Clone, Debug, Default)]
pub struct HostGaugeHits {
    applied: Vec<u32>,
}

impl HostGaugeHits {
    pub const fn new() -> Self {
        Self {
            applied: Vec::new(),
        }
    }

    pub fn judge(&mut self, facts: &GaugeHitFacts, hit: &AttackGaugeHit) -> GaugeHitVerdict {
        if hit.phase != facts.local_phase {
            return GaugeHitVerdict::Drop(GaugeHitDrop::OtherPhase);
        }
        let Some(serial) = facts.serial else {
            return GaugeHitVerdict::Drop(GaugeHitDrop::Closed);
        };
        if serial != hit.serial {
            return GaugeHitVerdict::Drop(GaugeHitDrop::OtherOpening);
        }
        let Some((hp, max_hp)) = facts.node_hp else {
            return GaugeHitVerdict::Drop(GaugeHitDrop::NoSuchNode);
        };
        if hp < 1 {
            return GaugeHitVerdict::Drop(GaugeHitDrop::NodeAlreadyBroken);
        }
        if self.applied.contains(&hit.seq) {
            return GaugeHitVerdict::Drop(GaugeHitDrop::Repeated);
        }
        if self.applied.len() >= GAUGE_HIT_SEQ_MEMORY_ENTRIES {
            self.applied.remove(0);
        }
        self.applied.push(hit.seq);
        let hp_after = node_hp_after_damage(hp, max_hp, hit.damage, facts.floor_percent);
        GaugeHitVerdict::Apply {
            hp_before: hp,
            hp_after,
            breaks: hp_after < 1,
        }
    }

    pub fn clear(&mut self) {
        self.applied.clear();
    }
}

/// A node's hp after `damage`, as the dummy's hit function sets it: the damage comes off, then a
/// non-zero floor percent keeps the node at that share of its max hp.
pub fn node_hp_after_damage(hp: i32, max_hp: i32, damage: i32, floor_percent: i32) -> i32 {
    let after = hp.saturating_sub(damage);
    if floor_percent == 0 {
        return after;
    }
    let floor = max_hp.saturating_mul(floor_percent) as f32 / ATTACK_GAUGE_PERCENT_DIVISOR;
    if (after as f32) < floor {
        floor as i32
    } else {
        after
    }
}

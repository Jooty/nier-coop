use crate::attack_gauge::GUEST_GAUGE_HIT_REPEAT_WINDOW_MS;

/// The guest's side of gauge hits: numbers each hit it sends and drops a swing that lands on the
/// same node again within the repeat window.
#[derive(Clone, Debug, Default)]
pub struct GuestGaugeHits {
    last_seq: u32,
    recent: Vec<RecentHit>,
}

#[derive(Clone, Copy, Debug)]
struct RecentHit {
    node: u8,
    hit_id: u32,
    at_ms: u64,
}

impl GuestGaugeHits {
    pub const fn new() -> Self {
        Self {
            last_seq: 0,
            recent: Vec::new(),
        }
    }

    pub fn take(&mut self, now_ms: u64, node: u8, hit_id: u32) -> Option<u32> {
        self.recent
            .retain(|hit| now_ms.saturating_sub(hit.at_ms) < GUEST_GAUGE_HIT_REPEAT_WINDOW_MS);
        if self
            .recent
            .iter()
            .any(|hit| hit.node == node && hit.hit_id == hit_id)
        {
            return None;
        }
        self.recent.push(RecentHit {
            node,
            hit_id,
            at_ms: now_ms,
        });
        self.last_seq = self.last_seq.wrapping_add(1);
        Some(self.last_seq)
    }
}

/// The damage the dummy's hit function took from a node the guest set to `measuring_hp` first,
/// or `None` when it took nothing.
pub fn measured_gauge_damage(measuring_hp: i32, after: i32) -> Option<i32> {
    let damage = measuring_hp.saturating_sub(after);
    (damage > 0).then_some(damage)
}

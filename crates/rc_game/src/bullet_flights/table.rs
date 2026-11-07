use crate::bullet_flights::{BULLET_FLIGHT_STALE_MS, MAX_PARTNER_BULLET_FLIGHTS};

#[derive(Debug, Default)]
pub struct BulletFlightTable {
    seen: Vec<(u16, u64)>,
}

impl BulletFlightTable {
    pub fn note(&mut self, instance: u16, now_ms: u64) -> bool {
        if let Some(entry) = self.seen.iter_mut().find(|(known, _)| *known == instance) {
            entry.1 = now_ms;
            return true;
        }
        if self.seen.len() >= MAX_PARTNER_BULLET_FLIGHTS {
            return false;
        }
        self.seen.push((instance, now_ms));
        true
    }

    pub fn forget(&mut self, instance: u16) {
        self.seen.retain(|(known, _)| *known != instance);
    }

    pub fn take_stale(&mut self, now_ms: u64) -> Vec<u16> {
        let stale: Vec<u16> = self
            .seen
            .iter()
            .filter(|(_, at)| now_ms.saturating_sub(*at) >= BULLET_FLIGHT_STALE_MS)
            .map(|(instance, _)| *instance)
            .collect();
        self.seen.retain(|(instance, _)| !stale.contains(instance));
        stale
    }

    pub fn take_all(&mut self) -> Vec<u16> {
        self.seen.drain(..).map(|(instance, _)| instance).collect()
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

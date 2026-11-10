use replicant_proto::BulletFlight;

use crate::enemy_bullets::{ENEMY_BULLET_STALE_MS, MAX_SHOWN_ENEMY_BULLETS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnemyBulletStep {
    Start,
    Move,
    Stop,
    Ignore,
    Full,
}

/// The guest's side: which host enemy bullets are on screen and when each last moved. A bullet
/// starts only from a flight that carries its look.
#[derive(Debug, Default)]
pub struct EnemyBulletBoard {
    live: Vec<(u16, u64)>,
}

impl EnemyBulletBoard {
    pub fn note(&mut self, flight: &BulletFlight, now_ms: u64) -> EnemyBulletStep {
        let known = self
            .live
            .iter()
            .position(|(instance, _)| *instance == flight.instance);
        match known {
            Some(index) if flight.ended => {
                self.live.remove(index);
                EnemyBulletStep::Stop
            }
            Some(index) => {
                self.live[index].1 = now_ms;
                EnemyBulletStep::Move
            }
            None if flight.ended || flight.look.is_none() => EnemyBulletStep::Ignore,
            None if self.live.len() >= MAX_SHOWN_ENEMY_BULLETS => EnemyBulletStep::Full,
            None => {
                self.live.push((flight.instance, now_ms));
                EnemyBulletStep::Start
            }
        }
    }

    pub fn forget(&mut self, instance: u16) {
        self.live.retain(|(known, _)| *known != instance);
    }

    pub fn take_stale(&mut self, now_ms: u64) -> Vec<u16> {
        let stale: Vec<u16> = self
            .live
            .iter()
            .filter(|(_, at)| now_ms.saturating_sub(*at) >= ENEMY_BULLET_STALE_MS)
            .map(|(instance, _)| *instance)
            .collect();
        self.live.retain(|(instance, _)| !stale.contains(instance));
        stale
    }

    pub fn take_all(&mut self) -> Vec<u16> {
        self.live.drain(..).map(|(instance, _)| instance).collect()
    }

    pub fn len(&self) -> usize {
        self.live.len()
    }

    pub fn is_empty(&self) -> bool {
        self.live.is_empty()
    }
}

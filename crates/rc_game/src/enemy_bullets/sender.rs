use rc_proto::{EnemyBulletFlight, EnemyId};

use crate::bullet_flights::{BulletTracker, SightedBullet};
use crate::enemy_bullets::EnemyBullet;

/// The host's side: turns each scan of enemy bullets into the flights to send, each tagged with
/// the enemy that fired it, an ended flight included.
#[derive(Debug, Default)]
pub struct EnemyBulletSender {
    tracker: BulletTracker,
    owners: Vec<(u16, EnemyId)>,
}

impl EnemyBulletSender {
    pub fn update(&mut self, sighted: &[EnemyBullet], now_ms: u64) -> Vec<EnemyBulletFlight> {
        let bullets: Vec<SightedBullet> = sighted.iter().map(|seen| seen.bullet).collect();
        let previous = core::mem::replace(
            &mut self.owners,
            sighted
                .iter()
                .map(|seen| (seen.bullet.slot, seen.enemy))
                .collect(),
        );
        self.tracker
            .update(&bullets, now_ms)
            .into_iter()
            .filter_map(|flight| {
                // An ended slot may already hold a new bullet, so its owner is the old one.
                let owners = if flight.ended {
                    &previous
                } else {
                    &self.owners
                };
                owner_of(owners, flight.instance).map(|enemy| EnemyBulletFlight { enemy, flight })
            })
            .collect()
    }

    pub fn end_all(&mut self) -> Vec<EnemyBulletFlight> {
        let owners = core::mem::take(&mut self.owners);
        self.tracker
            .end_all()
            .into_iter()
            .filter_map(|flight| {
                owner_of(&owners, flight.instance).map(|enemy| EnemyBulletFlight { enemy, flight })
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tracker.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracker.is_empty()
    }
}

fn owner_of(owners: &[(u16, EnemyId)], slot: u16) -> Option<EnemyId> {
    owners
        .iter()
        .find(|(known, _)| *known == slot)
        .map(|(_, enemy)| *enemy)
}

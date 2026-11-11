use std::collections::VecDeque;

use replicant_proto::{BulletFlight, EnemyBulletFlight, EnemyId};

use crate::bullet_flights::{BULLET_LOOK_REPEAT_EVERY_MESSAGES, BulletTracker, SightedBullet};
use crate::enemy_bullets::{EnemyBullet, MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN};

#[derive(Debug, Clone, Copy)]
struct SentBullet {
    slot: u16,
    messages: u32,
}

/// The host's side: turns each scan of enemy bullets into the flights to send, each tagged with
/// its enemy. A scan sends at most `MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN` moves, new bullets
/// first, then the ones that waited longest, and one end per bullet the guest was told of.
#[derive(Debug, Default)]
pub struct EnemyBulletSender {
    tracker: BulletTracker,
    owners: Vec<(u16, EnemyId)>,
    sent_longest_ago_first: VecDeque<SentBullet>,
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
        let (ended, moving): (Vec<BulletFlight>, Vec<BulletFlight>) = self
            .tracker
            .update(&bullets, now_ms)
            .into_iter()
            .partition(|flight| flight.ended);
        // An ended slot may already hold a new bullet, so its owner is the old one.
        let mut out = self.ends_the_guest_knows(ended, &previous);
        for slot in self.slots_due(&moving) {
            let (Some(flight), Some(seen)) = (
                moving.iter().find(|flight| flight.instance == slot),
                sighted.iter().find(|seen| seen.bullet.slot == slot),
            ) else {
                continue;
            };
            let messages = self.take_sent(slot).map_or(0, |sent| sent.messages);
            let with_look = messages.is_multiple_of(BULLET_LOOK_REPEAT_EVERY_MESSAGES);
            self.sent_longest_ago_first.push_back(SentBullet {
                slot,
                messages: messages.wrapping_add(1),
            });
            out.push(EnemyBulletFlight {
                enemy: seen.enemy,
                flight: BulletFlight {
                    look: with_look.then_some(seen.bullet.look),
                    ..*flight
                },
            });
        }
        out
    }

    pub fn end_all(&mut self) -> Vec<EnemyBulletFlight> {
        let owners = core::mem::take(&mut self.owners);
        let ended = self.tracker.end_all();
        self.ends_the_guest_knows(ended, &owners)
    }

    pub fn len(&self) -> usize {
        self.tracker.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracker.is_empty()
    }

    fn ends_the_guest_knows(
        &mut self,
        ended: Vec<BulletFlight>,
        owners: &[(u16, EnemyId)],
    ) -> Vec<EnemyBulletFlight> {
        ended
            .into_iter()
            .filter_map(|flight| {
                self.take_sent(flight.instance)?;
                owner_of(owners, flight.instance).map(|enemy| EnemyBulletFlight { enemy, flight })
            })
            .collect()
    }

    fn slots_due(&self, moving: &[BulletFlight]) -> Vec<u16> {
        let is_moving = |slot: u16| moving.iter().any(|flight| flight.instance == slot);
        let never_sent = moving.iter().map(|flight| flight.instance).filter(|slot| {
            !self
                .sent_longest_ago_first
                .iter()
                .any(|sent| sent.slot == *slot)
        });
        let waiting = self
            .sent_longest_ago_first
            .iter()
            .map(|sent| sent.slot)
            .filter(|slot| is_moving(*slot));
        never_sent
            .chain(waiting)
            .take(MAX_ENEMY_BULLET_MOVES_SENT_PER_SCAN)
            .collect()
    }

    fn take_sent(&mut self, slot: u16) -> Option<SentBullet> {
        let index = self
            .sent_longest_ago_first
            .iter()
            .position(|sent| sent.slot == slot)?;
        self.sent_longest_ago_first.remove(index)
    }
}

fn owner_of(owners: &[(u16, EnemyId)], slot: u16) -> Option<EnemyId> {
    owners
        .iter()
        .find(|(known, _)| *known == slot)
        .map(|(_, enemy)| *enemy)
}

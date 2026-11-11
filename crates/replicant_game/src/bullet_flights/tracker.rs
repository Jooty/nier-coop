use replicant_proto::{BULLET_TRANSFORM_FLOATS, BulletFlight, MAX_BULLET_SPEED_ABS};

use crate::bullet_flights::{BULLET_LOOK_REPEAT_EVERY_MESSAGES, SightedBullet};

const MILLIS_PER_SECOND: f32 = 1000.0;

#[derive(Debug)]
struct TrackedBullet {
    slot: u16,
    hit_id: i32,
    last_ms: u64,
    last_position: [f32; 3],
    velocity: [f32; 3],
    last_transform: [f32; BULLET_TRANSFORM_FLOATS],
    messages: u32,
}

#[derive(Debug, Default)]
pub struct BulletTracker {
    tracked: Vec<TrackedBullet>,
}

impl BulletTracker {
    pub fn update(&mut self, sighted: &[SightedBullet], now_ms: u64) -> Vec<BulletFlight> {
        let mut out = Vec::new();
        let gone: Vec<u16> = self
            .tracked
            .iter()
            .filter(|tracked| {
                !sighted
                    .iter()
                    .any(|bullet| bullet.slot == tracked.slot && bullet.hit_id == tracked.hit_id)
            })
            .map(|tracked| tracked.slot)
            .collect();
        for slot in gone {
            if let Some(ended) = self.end(slot) {
                out.push(ended);
            }
        }
        for bullet in sighted {
            let position = translation_of(&bullet.transform);
            let index = match self
                .tracked
                .iter()
                .position(|tracked| tracked.slot == bullet.slot)
            {
                Some(index) => index,
                None => {
                    self.tracked.push(TrackedBullet {
                        slot: bullet.slot,
                        hit_id: bullet.hit_id,
                        last_ms: now_ms,
                        last_position: position,
                        velocity: [0.0; 3],
                        last_transform: bullet.transform,
                        messages: 0,
                    });
                    self.tracked.len() - 1
                }
            };
            let tracked = &mut self.tracked[index];
            if tracked.messages > 0 {
                tracked.velocity = velocity_between(
                    tracked.last_position,
                    position,
                    now_ms.saturating_sub(tracked.last_ms),
                );
            }
            tracked.last_ms = now_ms;
            tracked.last_position = position;
            tracked.last_transform = bullet.transform;
            let with_look = tracked
                .messages
                .is_multiple_of(BULLET_LOOK_REPEAT_EVERY_MESSAGES);
            tracked.messages += 1;
            out.push(BulletFlight {
                instance: bullet.slot,
                transform: bullet.transform,
                velocity: tracked.velocity,
                look: with_look.then_some(bullet.look),
                ended: false,
            });
        }
        out
    }

    pub fn end_all(&mut self) -> Vec<BulletFlight> {
        let slots: Vec<u16> = self.tracked.iter().map(|tracked| tracked.slot).collect();
        slots
            .into_iter()
            .filter_map(|slot| self.end(slot))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tracked.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracked.is_empty()
    }

    fn end(&mut self, slot: u16) -> Option<BulletFlight> {
        let index = self
            .tracked
            .iter()
            .position(|tracked| tracked.slot == slot)?;
        let tracked = self.tracked.remove(index);
        Some(BulletFlight {
            instance: tracked.slot,
            transform: tracked.last_transform,
            velocity: [0.0; 3],
            look: None,
            ended: true,
        })
    }
}

fn translation_of(transform: &[f32; BULLET_TRANSFORM_FLOATS]) -> [f32; 3] {
    [transform[3], transform[7], transform[11]]
}

fn velocity_between(from: [f32; 3], to: [f32; 3], elapsed_ms: u64) -> [f32; 3] {
    if elapsed_ms == 0 {
        return [0.0; 3];
    }
    let seconds = elapsed_ms as f32 / MILLIS_PER_SECOND;
    core::array::from_fn(|axis| {
        ((to[axis] - from[axis]) / seconds).clamp(-MAX_BULLET_SPEED_ABS, MAX_BULLET_SPEED_ABS)
    })
}

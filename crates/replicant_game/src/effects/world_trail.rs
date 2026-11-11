use replicant_proto::WORLD_EFFECT_TRANSFORM_FLOATS;

pub const WORLD_EFFECT_MOVE_INTERVAL_MS: u64 = 50;
pub const MAX_TRAILED_WORLD_EFFECTS: usize = 32;
pub const WORLD_EFFECT_MOVE_EPSILON: f32 = 0.01;

type Transform = [f32; WORLD_EFFECT_TRANSFORM_FLOATS];

/// The world effects we sent, with the transform the partner last got, so a spell's moving
/// effect (a lance in flight) is followed by move messages.
#[derive(Debug, Default)]
pub struct WorldEffectTrail {
    entries: Vec<TrailEntry>,
}

#[derive(Debug)]
struct TrailEntry {
    handle: u32,
    sent: Transform,
    sent_ms: u64,
}

impl WorldEffectTrail {
    /// Starts following `handle`; the oldest entry is dropped (and returned) when full.
    pub fn track(&mut self, handle: u32, transform: Transform, now_ms: u64) -> Option<u32> {
        self.entries.retain(|entry| entry.handle != handle);
        let dropped = (self.entries.len() >= MAX_TRAILED_WORLD_EFFECTS)
            .then(|| self.entries.remove(0).handle);
        self.entries.push(TrailEntry {
            handle,
            sent: transform,
            sent_ms: now_ms,
        });
        dropped
    }

    /// The handles whose transform moved since it was last sent, at most once per interval.
    /// `current` reads a live effect's transform; None means it ended and it is forgotten.
    pub fn due_moves(
        &mut self,
        now_ms: u64,
        mut current: impl FnMut(u32) -> Option<Transform>,
    ) -> Vec<(u32, Transform)> {
        let mut moves = Vec::new();
        self.entries.retain_mut(|entry| {
            if now_ms.saturating_sub(entry.sent_ms) < WORLD_EFFECT_MOVE_INTERVAL_MS {
                return true;
            }
            let Some(transform) = current(entry.handle) else {
                return false;
            };
            if moved(&entry.sent, &transform) {
                entry.sent = transform;
                entry.sent_ms = now_ms;
                moves.push((entry.handle, transform));
            }
            true
        });
        moves
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

fn moved(before: &Transform, after: &Transform) -> bool {
    before
        .iter()
        .zip(after)
        .any(|(old, new)| (old - new).abs() > WORLD_EFFECT_MOVE_EPSILON)
}

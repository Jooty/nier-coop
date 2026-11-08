use crate::blocks::{
    BLOCK_FINAL_RESEND_DISTANCE, BLOCK_REMOTE_OWNERSHIP_MS, BLOCK_SEND_DISTANCE,
    BLOCK_SEND_INTERVAL_MS, BLOCK_SETTLE_MS, distance_between,
};

#[derive(Debug)]
struct Tracked {
    id: u16,
    last_sent: [f32; 3],
    last_sent_ms: u64,
    settle_until_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct BlockOutbox {
    tracked: Option<Tracked>,
    remote_move: Option<(u16, u64)>,
}

impl Tracked {
    fn send_if_due(&mut self, now_ms: u64, position: [f32; 3]) -> Option<[f32; 3]> {
        let due = now_ms.saturating_sub(self.last_sent_ms) >= BLOCK_SEND_INTERVAL_MS;
        if !due || distance_between(position, self.last_sent) <= BLOCK_SEND_DISTANCE {
            return None;
        }
        self.last_sent = position;
        self.last_sent_ms = now_ms;
        Some(position)
    }
}

impl BlockOutbox {
    pub fn tracking(&self) -> Option<u16> {
        self.tracked.as_ref().map(|tracked| tracked.id)
    }

    pub fn note_remote_move(&mut self, now_ms: u64, id: u16) {
        self.remote_move = Some((id, now_ms));
        if self.tracking() == Some(id) {
            self.tracked = None;
        }
    }

    pub fn observe(
        &mut self,
        now_ms: u64,
        held: Option<u16>,
        position_of: impl Fn(u16) -> Option<[f32; 3]>,
    ) -> Option<(u16, [f32; 3])> {
        match held {
            Some(id) => self.observe_held(now_ms, id, position_of(id)?),
            None => self.observe_settling(now_ms, position_of),
        }
    }

    fn observe_held(
        &mut self,
        now_ms: u64,
        id: u16,
        position: [f32; 3],
    ) -> Option<(u16, [f32; 3])> {
        if self.partner_owns(now_ms, id) {
            self.tracked = None;
            return None;
        }
        let Some(tracked) = self.tracked.as_mut().filter(|tracked| tracked.id == id) else {
            self.tracked = Some(Tracked {
                id,
                last_sent: position,
                last_sent_ms: now_ms,
                settle_until_ms: None,
            });
            return None;
        };
        tracked.settle_until_ms = None;
        tracked
            .send_if_due(now_ms, position)
            .map(|position| (id, position))
    }

    fn observe_settling(
        &mut self,
        now_ms: u64,
        position_of: impl Fn(u16) -> Option<[f32; 3]>,
    ) -> Option<(u16, [f32; 3])> {
        let tracked = self.tracked.as_mut()?;
        let until = *tracked
            .settle_until_ms
            .get_or_insert(now_ms + BLOCK_SETTLE_MS);
        let id = tracked.id;
        let position = position_of(id);
        if now_ms >= until {
            let last_sent = tracked.last_sent;
            self.tracked = None;
            return position
                .filter(|position| {
                    distance_between(*position, last_sent) > BLOCK_FINAL_RESEND_DISTANCE
                })
                .map(|position| (id, position));
        }
        tracked
            .send_if_due(now_ms, position?)
            .map(|position| (id, position))
    }

    fn partner_owns(&self, now_ms: u64, id: u16) -> bool {
        self.remote_move.is_some_and(|(remote_id, at_ms)| {
            remote_id == id && now_ms.saturating_sub(at_ms) < BLOCK_REMOTE_OWNERSHIP_MS
        })
    }
}

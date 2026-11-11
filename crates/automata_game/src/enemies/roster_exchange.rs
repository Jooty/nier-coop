use crate::enemies::{
    CHANGED_ROSTER_IS_SENT_AFTER_MS, ENEMIES_IN_ONE_ROSTER_PART, EnemyRosterPart, EnemySighting,
    MOST_ENEMIES_IN_ONE_ROSTER, UNCHANGED_ROSTER_IS_SENT_AGAIN_AFTER_MS,
};

/// Puts the parts of one round back together; a part of another round starts over.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RosterAssembly {
    round: Option<u32>,
    gathered: Vec<EnemySighting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RosterPacer {
    sent: Option<(Vec<EnemySighting>, u64)>,
    rounds_sent: u32,
}

impl RosterAssembly {
    /// The whole roster when this part closes its round.
    pub fn receive(&mut self, part: EnemyRosterPart) -> Option<Vec<EnemySighting>> {
        if self.round != Some(part.round) {
            self.round = Some(part.round);
            self.gathered.clear();
        }
        let room = MOST_ENEMIES_IN_ONE_ROSTER.saturating_sub(self.gathered.len());
        self.gathered.extend(part.enemies.into_iter().take(room));
        if !part.last {
            return None;
        }
        self.round = None;
        Some(core::mem::take(&mut self.gathered))
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }
}

impl RosterPacer {
    pub fn is_due(&self, now_ms: u64, current: &[EnemySighting]) -> bool {
        match &self.sent {
            None => true,
            Some((sent, at_ms)) => {
                let waited = now_ms.saturating_sub(*at_ms);
                if sent.as_slice() == current {
                    waited >= UNCHANGED_ROSTER_IS_SENT_AGAIN_AFTER_MS
                } else {
                    waited >= CHANGED_ROSTER_IS_SENT_AFTER_MS
                }
            }
        }
    }

    /// The parts to send for this roster, under a round number no earlier send used.
    pub fn parts_to_send(
        &mut self,
        now_ms: u64,
        current: &[EnemySighting],
    ) -> Vec<EnemyRosterPart> {
        self.rounds_sent = self.rounds_sent.wrapping_add(1);
        self.sent = Some((current.to_vec(), now_ms));
        roster_parts(self.rounds_sent, current)
    }

    pub fn forget(&mut self) {
        self.sent = None;
    }
}

/// At least one part, so an empty roster is told too; enemies past the roster limit are left out.
pub fn roster_parts(round: u32, enemies: &[EnemySighting]) -> Vec<EnemyRosterPart> {
    let sent = &enemies[..enemies.len().min(MOST_ENEMIES_IN_ONE_ROSTER)];
    let chunks: Vec<&[EnemySighting]> = sent.chunks(ENEMIES_IN_ONE_ROSTER_PART).collect();
    if chunks.is_empty() {
        return vec![EnemyRosterPart {
            round,
            last: true,
            enemies: Vec::new(),
        }];
    }
    let last_index = chunks.len() - 1;
    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| EnemyRosterPart {
            round,
            last: index == last_index,
            enemies: chunk.to_vec(),
        })
        .collect()
}

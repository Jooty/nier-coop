use crate::blocks::MAX_STATE_DIFF_LINES;
use crate::offsets::{ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateChange {
    pub offset: usize,
    pub before: u32,
    pub after: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDiff {
    pub from_stage: &'static str,
    pub to_stage: &'static str,
    pub changes: Vec<StateChange>,
}

#[derive(Debug)]
pub struct StateTrail {
    stage: &'static str,
    bytes: Vec<u8>,
}

impl StateTrail {
    pub fn new(stage: &'static str, bytes: Vec<u8>) -> Self {
        Self { stage, bytes }
    }

    pub fn stage(&self) -> &'static str {
        self.stage
    }

    pub fn advance(&mut self, stage: &'static str, bytes: Vec<u8>) -> Option<StateDiff> {
        if stage == self.stage {
            return None;
        }
        let diff = StateDiff {
            from_stage: self.stage,
            to_stage: stage,
            changes: changed_words(&self.bytes, &bytes),
        };
        self.stage = stage;
        self.bytes = bytes;
        Some(diff)
    }
}

pub fn changed_words(before: &[u8], after: &[u8]) -> Vec<StateChange> {
    let transform = ACTOR_TRANSFORM_BLOCK_OFFSET as usize
        ..ACTOR_TRANSFORM_BLOCK_OFFSET as usize + ACTOR_TRANSFORM_BLOCK_BYTES;
    let word = |bytes: &[u8], offset: usize| {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    };
    let length = before.len().min(after.len()) / 4 * 4;
    (0..length)
        .step_by(4)
        .filter(|offset| !transform.contains(offset))
        .filter_map(|offset| {
            let (old, new) = (word(before, offset), word(after, offset));
            (old != new).then_some(StateChange {
                offset,
                before: old,
                after: new,
            })
        })
        .take(MAX_STATE_DIFF_LINES)
        .collect()
}

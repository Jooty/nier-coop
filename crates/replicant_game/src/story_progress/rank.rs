use replicant_proto::StoryArray;

use crate::story_progress::{ROUTE_CYCLE_FLAG_IDS, StoryArrays, milestone_ids};

const PART_LETTERS: &[u8] = b"ABCDE";

/// How far a story state is: the route cycle (set ending flags), then the phase name's part
/// letter, then the count of set milestone flags. A part letter outside A to E is not compared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoryRank {
    pub cycle: u8,
    pub part: Option<u8>,
    pub milestones: u32,
}

impl StoryRank {
    pub fn of(phase_name: &[u8], arrays: &StoryArrays) -> Self {
        let milestones = [StoryArray::GameFlags, StoryArray::SnowFlags]
            .into_iter()
            .map(|array| arrays.set_flags_among(array, milestone_ids(array)))
            .sum();
        let cycle = arrays.set_flags_among(StoryArray::GameFlags, &ROUTE_CYCLE_FLAG_IDS);
        Self {
            cycle: u8::try_from(cycle).unwrap_or(u8::MAX),
            part: part_of_phase(phase_name),
            milestones,
        }
    }

    pub fn outranks(&self, other: &Self) -> bool {
        if self.cycle != other.cycle {
            return self.cycle > other.cycle;
        }
        match (self.part, other.part) {
            (Some(mine), Some(theirs)) if mine != theirs => mine > theirs,
            _ => self.milestones > other.milestones,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.milestones == 0
    }
}

/// The part letter of a phase name as 1 (A) to 5 (E), or `None` for any other first letter.
pub fn part_of_phase(phase_name: &[u8]) -> Option<u8> {
    let first = phase_name.first()?.to_ascii_uppercase();
    let position = PART_LETTERS.iter().position(|&letter| letter == first)?;
    u8::try_from(position + 1).ok()
}

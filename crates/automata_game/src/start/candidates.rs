use crate::start::{
    BASE_NAME_PART, CONDITIONAL_GROUP, ChapterRecord, OPENING_MISSION_NAME_PART,
    OPENING_MISSION_PHASE_ID,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub index: usize,
    pub record_index: usize,
    pub record: ChapterRecord,
}

pub fn candidates(records: &[ChapterRecord]) -> Vec<Candidate> {
    let mut found = Vec::new();
    let mut previous_group = None;
    for (record_index, record) in records.iter().enumerate() {
        let is_group_header = previous_group != Some(record.group);
        previous_group = Some(record.group);
        if is_group_header && is_selectable_start(record) {
            found.push(Candidate {
                index: found.len(),
                record_index,
                record: record.clone(),
            });
        }
    }
    found
}

fn is_selectable_start(record: &ChapterRecord) -> bool {
    let first_character_selectable = record.hero_mask & 1 != 0;
    let lowered_name = record.phase_name.to_ascii_lowercase();
    let is_opening_mission = record.phase_id == OPENING_MISSION_PHASE_ID
        && lowered_name.contains(&OPENING_MISSION_NAME_PART.to_ascii_lowercase());
    let is_base = lowered_name.contains(&BASE_NAME_PART.to_ascii_lowercase());
    first_character_selectable
        && record.group != CONDITIONAL_GROUP
        && !is_opening_mission
        && !is_base
}

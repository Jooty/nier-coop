use crate::story_steps::StepClassRow;

/// A done the detour saw this copy's game set on a blocking story action (before it is written).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedDone {
    pub id: u32,
    pub class: Option<&'static StepClassRow>,
    pub vftable_rva: u32,
    pub value: i32,
    pub ours: bool,
}

impl ObservedDone {
    pub fn is_mirrored(&self) -> bool {
        self.class.is_some_and(|row| row.sort.is_mirrored())
    }

    pub fn class_name(&self) -> &'static str {
        self.class.map_or("(not in the table)", |row| row.name)
    }
}

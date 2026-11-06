use crate::phases::PHASE_TABLE;

#[derive(Debug, PartialEq, Eq)]
pub struct PhaseEntry {
    pub name: &'static str,
    pub area: &'static str,
}

impl PhaseEntry {
    pub fn find(name: &str) -> Option<&'static PhaseEntry> {
        PHASE_TABLE.iter().find(|entry| entry.name == name)
    }

    pub fn index_of(name: &str) -> Option<usize> {
        PHASE_TABLE.iter().position(|entry| entry.name == name)
    }
}

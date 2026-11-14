use crate::exe::ExeVariant;
use crate::story_steps::{StepClassRow, step_class_table};

/// The class table sorted by one exe's vftable RVAs, for a lookup without allocation.
#[derive(Debug, Clone, Default)]
pub struct StepClassLookup {
    by_vftable_rva: Vec<(u32, &'static StepClassRow)>,
}

impl StepClassLookup {
    pub fn for_variant(variant: ExeVariant) -> Self {
        Self::of_rows(step_class_table(), variant)
    }

    pub fn of_rows(rows: &'static [StepClassRow], variant: ExeVariant) -> Self {
        let mut by_vftable_rva: Vec<(u32, &'static StepClassRow)> = rows
            .iter()
            .map(|row| (row.vftable.rva(variant), row))
            .collect();
        by_vftable_rva.sort_by_key(|(rva, _)| *rva);
        Self { by_vftable_rva }
    }

    pub fn row_of_vftable_rva(&self, rva: u32) -> Option<&'static StepClassRow> {
        self.by_vftable_rva
            .binary_search_by_key(&rva, |(at, _)| *at)
            .ok()
            .map(|index| self.by_vftable_rva[index].1)
    }

    pub fn len(&self) -> usize {
        self.by_vftable_rva.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_vftable_rva.is_empty()
    }
}

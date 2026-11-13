use std::sync::LazyLock;

use crate::story_steps::StepClassRow;

const STEP_CLASSES_TSV: &str = include_str!("step_classes.tsv");

static STEP_CLASS_TABLE: LazyLock<Vec<StepClassRow>> =
    LazyLock::new(|| data_lines().filter_map(StepClassRow::of_tsv_line).collect());

/// Every row of `step_classes.tsv` that reads as a row.
pub fn step_class_table() -> &'static [StepClassRow] {
    &STEP_CLASS_TABLE
}

/// The data lines of `step_classes.tsv` that do not read as a row (a test keeps this empty).
pub fn step_class_table_rows_unread() -> Vec<&'static str> {
    data_lines()
        .filter(|line| StepClassRow::of_tsv_line(line).is_none())
        .collect()
}

fn data_lines() -> impl Iterator<Item = &'static str> {
    STEP_CLASSES_TSV
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
}

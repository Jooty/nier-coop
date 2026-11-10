use crate::puppet::MotionEntry;

pub const IDLE_NAME_FALLBACK_PART: &str = "idle_lp";

pub fn idle_index_from_table(table: &[MotionEntry]) -> Option<i32> {
    table
        .iter()
        .find(|entry| entry.name.contains(IDLE_NAME_FALLBACK_PART))
        .map(|entry| entry.index)
}

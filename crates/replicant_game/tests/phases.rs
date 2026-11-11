use std::collections::BTreeSet;

use replicant_game::offsets::PHASE_NAME_BUFFER_LEN;
use replicant_game::{PHASE_TABLE, PhaseEntry};

#[test]
fn table_holds_the_94_phases_with_a_script_pack() {
    assert_eq!(PHASE_TABLE.len(), 94);
}

#[test]
fn names_are_unique_and_fit_the_game_buffer() {
    let names: BTreeSet<&str> = PHASE_TABLE.iter().map(|entry| entry.name).collect();
    assert_eq!(names.len(), PHASE_TABLE.len());
    for entry in &PHASE_TABLE {
        assert!(entry.name.len() < PHASE_NAME_BUFFER_LEN, "{}", entry.name);
        assert!(entry.name.is_ascii(), "{}", entry.name);
    }
}

#[test]
fn the_phase_without_a_pack_is_excluded() {
    assert_eq!(PhaseEntry::find("A_TGS2009_01"), None);
}

#[test]
fn new_game_phase_is_found_by_name() {
    let entry = PhaseEntry::find("A_TOKYO_FIELD_01").expect("new game phase is in the table");
    assert_eq!(entry.area, "Tokyo field (prologue) 01");
}

#[test]
fn lookup_is_exact() {
    assert_eq!(PhaseEntry::find("a_tokyo_field_01"), None);
    assert_eq!(PhaseEntry::find("A_TOKYO_FIELD_01 "), None);
}

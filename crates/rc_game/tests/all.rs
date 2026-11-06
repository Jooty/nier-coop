mod fake_memory;

#[path = "actor_model.rs"]
mod actor_model;
#[path = "entity.rs"]
mod entity;
#[path = "inventory.rs"]
mod inventory;
#[path = "invincibility.rs"]
mod invincibility;
#[path = "noclip.rs"]
mod noclip;
#[path = "patches.rs"]
mod patches;
#[path = "pe_identity.rs"]
mod pe_identity;
#[path = "phase_state.rs"]
mod phase_state;
#[path = "phases.rs"]
mod phases;
#[path = "player_stats.rs"]
mod player_stats;
#[path = "position.rs"]
mod position;

use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn every_test_file_is_part_of_the_suite() {
    let tests_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let suite = include_str!("all.rs");
    let missing: BTreeSet<String> = std::fs::read_dir(&tests_dir)
        .expect("tests directory is readable")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|name| name.strip_suffix(".rs").map(str::to_owned))
        .filter(|name| name != "all" && !suite.contains(&format!("mod {name};")))
        .collect();
    assert!(
        missing.is_empty(),
        "test files missing from tests/all.rs: {missing:?}"
    );
}

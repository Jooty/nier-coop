#[path = "dialogue_wire.rs"]
mod dialogue_wire;
#[path = "effect_wire.rs"]
mod effect_wire;
#[path = "enemy_wire.rs"]
mod enemy_wire;
#[path = "follow_phase.rs"]
mod follow_phase;
#[path = "music_wire.rs"]
mod music_wire;
#[path = "npc_wire.rs"]
mod npc_wire;
#[path = "sky_wire.rs"]
mod sky_wire;
#[path = "slot.rs"]
mod slot;
#[path = "track.rs"]
mod track;
#[path = "wire.rs"]
mod wire;

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

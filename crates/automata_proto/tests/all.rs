#[path = "defeat_wire.rs"]
mod defeat_wire;
#[path = "event_wire.rs"]
mod event_wire;
#[path = "generator_counts_wire.rs"]
mod generator_counts_wire;
#[path = "member_wire.rs"]
mod member_wire;
#[path = "pickup_wire.rs"]
mod pickup_wire;
#[path = "save_wire.rs"]
mod save_wire;
#[path = "skip_vote_wire.rs"]
mod skip_vote_wire;
#[path = "story_step_wire.rs"]
mod story_step_wire;
#[path = "story_wire.rs"]
mod story_wire;
#[path = "watch_wire.rs"]
mod watch_wire;
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

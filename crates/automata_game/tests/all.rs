#[path = "addresses.rs"]
mod addresses;
#[path = "autopilot.rs"]
mod autopilot;
#[path = "build_gate.rs"]
mod build_gate;
#[path = "characters.rs"]
mod characters;
#[path = "engine_partner.rs"]
mod engine_partner;
#[path = "exe_identity.rs"]
mod exe_identity;
#[path = "focus.rs"]
mod focus;
#[path = "free_rule.rs"]
mod free_rule;
#[path = "input_block.rs"]
mod input_block;
#[path = "load_gate.rs"]
mod load_gate;
#[path = "partner_machine.rs"]
mod partner_machine;
#[path = "partner_rules.rs"]
mod partner_rules;
#[path = "pod_aim.rs"]
mod pod_aim;
#[path = "save_redirect.rs"]
mod save_redirect;
#[path = "second_copy.rs"]
mod second_copy;
#[path = "start_candidates.rs"]
mod start_candidates;
#[path = "start_machine.rs"]
mod start_machine;
#[path = "start_plan.rs"]
mod start_plan;
#[path = "start_records.rs"]
mod start_records;
#[path = "story_gate.rs"]
mod story_gate;
#[path = "travel.rs"]
mod travel;

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

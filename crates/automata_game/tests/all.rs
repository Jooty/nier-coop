#[path = "addresses.rs"]
mod addresses;
#[path = "autopilot.rs"]
mod autopilot;
#[path = "boot_screen.rs"]
mod boot_screen;
#[path = "build_gate.rs"]
mod build_gate;
#[path = "characters.rs"]
mod characters;
#[path = "co_op_log.rs"]
mod co_op_log;
#[path = "defeat.rs"]
mod defeat;
#[path = "downed.rs"]
mod downed;
#[path = "downed_item_hold.rs"]
mod downed_item_hold;
#[path = "enemies.rs"]
mod enemies;
#[path = "enemy_lead.rs"]
mod enemy_lead;
#[path = "enemy_spawns.rs"]
mod enemy_spawns;
#[path = "engine_partner.rs"]
mod engine_partner;
#[path = "event_fixes.rs"]
mod event_fixes;
#[path = "event_follow.rs"]
mod event_follow;
#[path = "exe_identity.rs"]
mod exe_identity;
#[path = "extra_body_roster.rs"]
mod extra_body_roster;
#[path = "focus.rs"]
mod focus;
#[path = "free_rule.rs"]
mod free_rule;
#[path = "generator_counts.rs"]
mod generator_counts;
#[path = "generator_sets.rs"]
mod generator_sets;
#[path = "held_bodies.rs"]
mod held_bodies;
#[path = "hud_look.rs"]
mod hud_look;
#[path = "input_block.rs"]
mod input_block;
#[path = "leave_test.rs"]
mod leave_test;
#[path = "load_gate.rs"]
mod load_gate;
#[path = "load_gate_places.rs"]
mod load_gate_places;
#[path = "load_watch.rs"]
mod load_watch;
#[path = "minimap_dot.rs"]
mod minimap_dot;
#[path = "minute_line_budget.rs"]
mod minute_line_budget;
#[path = "new_game_start.rs"]
mod new_game_start;
#[path = "own_dash_cut.rs"]
mod own_dash_cut;
#[path = "own_input.rs"]
mod own_input;
#[path = "partner_flight.rs"]
mod partner_flight;
#[path = "partner_flight_take_over.rs"]
mod partner_flight_take_over;
#[path = "partner_hud.rs"]
mod partner_hud;
#[path = "partner_machine.rs"]
mod partner_machine;
#[path = "partner_rules.rs"]
mod partner_rules;
#[path = "pickups.rs"]
mod pickups;
#[path = "player_entries.rs"]
mod player_entries;
#[path = "player_inputs.rs"]
mod player_inputs;
#[path = "pod_aim.rs"]
mod pod_aim;
#[path = "route_lead.rs"]
mod route_lead;
#[path = "save_redirect.rs"]
mod save_redirect;
#[path = "save_together.rs"]
mod save_together;
#[path = "second_copy.rs"]
mod second_copy;
#[path = "session_mode.rs"]
mod session_mode;
#[path = "shared_defeat.rs"]
mod shared_defeat;
#[path = "skip_vote.rs"]
mod skip_vote;
#[path = "start_candidates.rs"]
mod start_candidates;
#[path = "start_machine.rs"]
mod start_machine;
#[path = "start_plan.rs"]
mod start_plan;
#[path = "start_records.rs"]
mod start_records;
#[path = "story_follow.rs"]
mod story_follow;
#[path = "story_gate.rs"]
mod story_gate;
#[path = "story_place.rs"]
mod story_place;
#[path = "story_steps.rs"]
mod story_steps;
#[path = "system_data_band.rs"]
mod system_data_band;
#[path = "travel.rs"]
mod travel;
#[path = "unforced_defeat.rs"]
mod unforced_defeat;

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

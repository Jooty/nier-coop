mod fake_memory;

#[path = "actor_model.rs"]
mod actor_model;
#[path = "arrival.rs"]
mod arrival;
#[path = "containers.rs"]
mod containers;
#[path = "cutscene_skip.rs"]
mod cutscene_skip;
#[path = "dialogue.rs"]
mod dialogue;
#[path = "door_sync.rs"]
mod door_sync;
#[path = "downed.rs"]
mod downed;
#[path = "effects.rs"]
mod effects;
#[path = "enemies.rs"]
mod enemies;
#[path = "enemy_combat.rs"]
mod enemy_combat;
#[path = "enemy_difficulty.rs"]
mod enemy_difficulty;
#[path = "entity.rs"]
mod entity;
#[path = "event_log.rs"]
mod event_log;
#[path = "focus.rs"]
mod focus;
#[path = "frame_ring.rs"]
mod frame_ring;
#[path = "inventory.rs"]
mod inventory;
#[path = "invincibility.rs"]
mod invincibility;
#[path = "local_pose.rs"]
mod local_pose;
#[path = "minimap.rs"]
mod minimap;
#[path = "music.rs"]
mod music;
#[path = "noclip.rs"]
mod noclip;
#[path = "npcs.rs"]
mod npcs;
#[path = "page_protection.rs"]
mod page_protection;
#[path = "partner_marker.rs"]
mod partner_marker;
#[path = "party_frame.rs"]
mod party_frame;
#[path = "patches.rs"]
mod patches;
#[path = "pe_identity.rs"]
mod pe_identity;
#[path = "phase_state.rs"]
mod phase_state;
#[path = "phases.rs"]
mod phases;
#[path = "pickups.rs"]
mod pickups;
#[path = "player_name.rs"]
mod player_name;
#[path = "player_stats.rs"]
mod player_stats;
#[path = "position.rs"]
mod position;
#[path = "puppet.rs"]
mod puppet;
#[path = "puppet_weapon.rs"]
mod puppet_weapon;
#[path = "readiness.rs"]
mod readiness;
#[path = "save_files.rs"]
mod save_files;
#[path = "sky.rs"]
mod sky;
#[path = "stay_together.rs"]
mod stay_together;
#[path = "story_calls.rs"]
mod story_calls;
#[path = "story_progress.rs"]
mod story_progress;
#[path = "talk_refusal.rs"]
mod talk_refusal;
#[path = "time_scale.rs"]
mod time_scale;
#[path = "ui_scenes.rs"]
mod ui_scenes;
#[path = "zone_rules.rs"]
mod zone_rules;

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

use rc_game::enemies::{
    CopyStance, CopyStanceLog, DyingBodies, holds_copy, kept_step, stop_own_step,
};
use rc_game::offsets::{ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, ACTOR_PENDING_DISPLACEMENT_OFFSET};
use rc_proto::EnemyId;

use crate::fake_memory::FakeMemory;

const ACTOR_ADDRESS: u64 = 0x2_0000_0000;
const ACTOR_BYTES: usize = 0x300;

fn id(ordinal: u16) -> EnemyId {
    EnemyId { group: 3, ordinal }
}

fn actor_with_step(step: [f32; 3], pending: bool) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(ACTOR_ADDRESS, &[0_u8; ACTOR_BYTES]);
    let bytes: Vec<u8> = step.iter().flat_map(|lane| lane.to_le_bytes()).collect();
    memory.put(ACTOR_ADDRESS + ACTOR_PENDING_DISPLACEMENT_OFFSET, &bytes);
    memory.put(
        ACTOR_ADDRESS + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET,
        &[u8::from(pending)],
    );
    memory
}

fn step_of(memory: &FakeMemory) -> [f32; 3] {
    let bytes = memory.get(ACTOR_ADDRESS + ACTOR_PENDING_DISPLACEMENT_OFFSET, 12);
    std::array::from_fn(|lane| {
        f32::from_le_bytes([
            bytes[lane * 4],
            bytes[lane * 4 + 1],
            bytes[lane * 4 + 2],
            bytes[lane * 4 + 3],
        ])
    })
}

fn flag_of(memory: &FakeMemory) -> u8 {
    memory.get(ACTOR_ADDRESS + ACTOR_DISPLACEMENT_PENDING_FLAG_OFFSET, 1)[0]
}

#[test]
fn a_held_copy_keeps_no_part_of_its_own_step() {
    assert_eq!(kept_step([0.4, -0.2, 0.3], false), [0.0, 0.0, 0.0]);
}

#[test]
fn a_dead_copy_keeps_only_its_fall() {
    assert_eq!(kept_step([0.4, -0.2, 0.3], true), [0.0, -0.2, 0.0]);
    assert_eq!(kept_step([0.4, 0.5, 0.3], true), [0.0, 0.0, 0.0]);
    assert_eq!(kept_step([0.4, f32::NAN, 0.3], true), [0.0, 0.0, 0.0]);
}

#[test]
fn stopping_a_held_copy_clears_its_step_and_reports_the_slide() {
    let mut memory = actor_with_step([0.3, -0.1, 0.4], true);
    let cut = stop_own_step(&mut memory, ACTOR_ADDRESS, false).expect("actor memory is readable");
    assert!(cut.is_some_and(|length| (length - 0.5).abs() < 1e-5));
    assert_eq!(step_of(&memory), [0.0, 0.0, 0.0]);
    assert_eq!(flag_of(&memory), 0);
}

#[test]
fn stopping_a_dead_copy_lets_it_fall_but_not_slide() {
    let mut memory = actor_with_step([0.3, -0.1, 0.4], true);
    stop_own_step(&mut memory, ACTOR_ADDRESS, true).expect("actor memory is readable");
    assert_eq!(step_of(&memory), [0.0, -0.1, 0.0]);
    assert_eq!(flag_of(&memory), 1);
}

#[test]
fn a_copy_with_no_pending_step_is_left_alone() {
    let mut memory = actor_with_step([0.3, 0.0, 0.4], false);
    let cut = stop_own_step(&mut memory, ACTOR_ADDRESS, false).expect("actor memory is readable");
    assert_eq!(cut, None);
    assert_eq!(step_of(&memory), [0.3, 0.0, 0.4]);
}

#[test]
fn each_stance_of_a_copy_is_announced_once_per_phase() {
    let mut log = CopyStanceLog::new();
    assert!(log.first_time(5, id(0), CopyStance::DeadStopped));
    assert!(!log.first_time(5, id(0), CopyStance::DeadStopped));
    assert!(log.first_time(5, id(0), CopyStance::HiddenWithHostBody));
    assert!(log.first_time(5, id(1), CopyStance::DeadStopped));
    assert!(log.first_time(6, id(0), CopyStance::DeadStopped));
}

#[test]
fn a_host_body_is_reported_gone_once_with_the_time_since_death() {
    let mut bodies = DyingBodies::new();
    bodies.track(5, id(0), 0x100, 7, 1000);
    bodies.track(5, id(1), 0x200, 8, 1500);
    assert!(bodies.gone(5, 2000, |_, _| true).is_empty());
    assert_eq!(
        bodies.gone(5, 6000, |actor, _| actor != 0x100),
        vec![(id(0), 5000)]
    );
    assert!(bodies.gone(5, 7000, |actor, _| actor != 0x100).is_empty());
    assert_eq!(bodies.len(), 1);
}

#[test]
fn a_new_phase_forgets_the_host_bodies() {
    let mut bodies = DyingBodies::new();
    bodies.track(5, id(0), 0x100, 7, 1000);
    assert!(bodies.gone(6, 2000, |_, _| false).is_empty());
    assert!(bodies.is_empty());
}

#[test]
fn a_copy_is_held_while_the_host_motion_is_known() {
    assert!(holds_copy(true));
}

#[test]
fn a_copy_without_a_readable_host_motion_runs_its_own() {
    assert!(!holds_copy(false));
}

#[test]
fn the_own_motion_line_names_the_copy_for_any_enemy() {
    let text = CopyStance::RunsOwnMotion.text("1001:0", 0.0);
    assert!(text.starts_with("copy 1001:0 follows motion: none"));
    assert!(!text.contains("boss"));
}

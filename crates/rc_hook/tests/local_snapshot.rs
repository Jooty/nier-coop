use rc_hook::net::LocalSnapshot;

fn snapshot() -> LocalSnapshot {
    LocalSnapshot {
        phase: 7,
        position: [1.0, 2.0, 3.0],
        yaw_radians: 0.5,
        hp: 40,
        max_hp: 100,
        body_motion_index: Some(889),
        body_motion_frame: 12.5,
    }
}

#[test]
fn a_normal_snapshot_passes_validation_unchanged() {
    let state = snapshot().to_player_state(9);
    assert_eq!(state.tick, 9);
    assert_eq!(state.phase, 7);
    assert_eq!(state.hp, 40);
    assert_eq!(state.max_hp, 100);
    assert_eq!(state.animation_id, 889);
    assert!((state.animation_seconds - 12.5).abs() < f32::EPSILON);
    assert_eq!(state.validate(), Ok(()));
}

#[test]
fn negative_or_oversized_hp_is_clamped_to_something_valid() {
    let mut low = snapshot();
    low.hp = -5;
    assert_eq!(low.to_player_state(1).hp, 0);
    let mut high = snapshot();
    high.hp = 500;
    let state = high.to_player_state(1);
    assert_eq!(state.hp, state.max_hp);
    assert_eq!(state.validate(), Ok(()));
}

#[test]
fn missing_motion_and_bad_frames_become_zero() {
    let mut none = snapshot();
    none.body_motion_index = None;
    none.body_motion_frame = f32::NAN;
    let state = none.to_player_state(1);
    assert_eq!(state.animation_id, 0);
    assert_eq!(state.animation_seconds, 0.0);
    assert_eq!(state.validate(), Ok(()));
}

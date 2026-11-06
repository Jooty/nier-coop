use rc_game::puppet::LayerMotion;
use rc_hook::net::LocalSnapshot;

fn snapshot() -> LocalSnapshot {
    LocalSnapshot {
        phase: 7,
        position: [1.0, 2.0, 3.0],
        yaw_radians: 0.5,
        hp: 40,
        max_hp: 100,
        body_motion: Some(LayerMotion {
            index: 889,
            frame: 12.5,
            speed: 1.0,
            looping: true,
        }),
    }
}

#[test]
fn a_normal_snapshot_passes_validation_unchanged() {
    let state = snapshot().to_player_state(9);
    assert_eq!(state.tick, 9);
    assert_eq!(state.phase, 7);
    assert_eq!(state.hp, 40);
    assert_eq!(state.max_hp, 100);
    let motion = state.motion.expect("motion rides along");
    assert_eq!(motion.index, 889);
    assert!((motion.frame - 12.5).abs() < f32::EPSILON);
    assert!(motion.looping);
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
fn missing_or_bad_motion_is_dropped_and_extremes_are_clamped() {
    let mut none = snapshot();
    none.body_motion = None;
    assert_eq!(none.to_player_state(1).motion, None);
    let mut bad = snapshot();
    bad.body_motion = Some(LayerMotion {
        index: 5,
        frame: f32::NAN,
        speed: 1.0,
        looping: false,
    });
    assert_eq!(bad.to_player_state(1).motion, None);
    let mut huge = snapshot();
    huge.body_motion = Some(LayerMotion {
        index: 5,
        frame: -3.0,
        speed: 900.0,
        looping: false,
    });
    assert_eq!(huge.to_player_state(1).validate(), Ok(()));
}

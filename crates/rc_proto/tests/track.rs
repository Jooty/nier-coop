use rc_proto::{MotionState, PlayerState, StateTrack};

fn state(tick: u32, x: f32, yaw: f32) -> PlayerState {
    PlayerState {
        tick,
        phase: 1,
        position: [x, 0.0, 0.0],
        yaw_radians: yaw,
        motion: Some(MotionState {
            index: tick as u16,
            frame: 0.0,
            speed: 1.0,
            looping: true,
        }),
        hp: 10,
        max_hp: 10,
    }
}

#[test]
fn position_is_interpolated_between_the_surrounding_states() {
    let mut track = StateTrack::new();
    track.push(1000, state(1, 0.0, 0.0));
    track.push(1100, state(2, 10.0, 0.0));
    let sample = track.sample(1150, 100).expect("sample");
    assert!((sample.position[0] - 5.0).abs() < 1e-4);
}

#[test]
fn yaw_takes_the_short_way_round() {
    let mut track = StateTrack::new();
    track.push(0, state(1, 0.0, 3.0));
    track.push(100, state(2, 0.0, -3.0));
    let sample = track.sample(150, 100).expect("sample");
    assert!(sample.yaw_radians.cos() < -0.9);
}

#[test]
fn the_newest_state_is_held_not_extrapolated() {
    let mut track = StateTrack::new();
    track.push(0, state(1, 0.0, 0.0));
    track.push(100, state(2, 10.0, 0.0));
    let sample = track.sample(900, 100).expect("sample");
    assert!((sample.position[0] - 10.0).abs() < 1e-4);
}

#[test]
fn before_the_first_state_the_first_is_used() {
    let mut track = StateTrack::new();
    track.push(500, state(1, 3.0, 0.0));
    let sample = track.sample(510, 100).expect("sample");
    assert!((sample.position[0] - 3.0).abs() < 1e-4);
}

#[test]
fn an_empty_track_has_no_sample() {
    assert!(StateTrack::new().sample(100, 100).is_none());
}

#[test]
fn repeated_and_stale_ticks_are_ignored() {
    let mut track = StateTrack::new();
    assert!(track.push(0, state(5, 0.0, 0.0)));
    assert!(!track.push(10, state(5, 1.0, 0.0)));
    assert!(!track.push(20, state(4, 1.0, 0.0)));
    assert_eq!(track.len(), 1);
}

#[test]
fn a_restarted_peer_replaces_the_old_stream() {
    let mut track = StateTrack::new();
    track.push(0, state(5000, 0.0, 0.0));
    assert!(track.push(33, state(1, 9.0, 0.0)));
    assert_eq!(track.len(), 1);
}

#[test]
fn a_phase_change_snaps_instead_of_sliding() {
    let mut track = StateTrack::new();
    track.push(0, state(1, 0.0, 0.0));
    track.push(
        100,
        PlayerState {
            phase: 2,
            ..state(2, 500.0, 0.0)
        },
    );
    let sample = track.sample(150, 100).expect("sample");
    assert_eq!(sample.phase, 1);
    assert!(sample.position[0].abs() < 1e-4);
}

#[test]
fn old_history_is_dropped() {
    let mut track = StateTrack::new();
    for tick in 0..40 {
        track.push(u64::from(tick) * 100, state(tick + 1, 0.0, 0.0));
    }
    assert!(track.len() <= 8);
}

#[test]
fn the_motion_comes_from_the_earlier_state() {
    let mut track = StateTrack::new();
    track.push(0, state(1, 0.0, 0.0));
    track.push(100, state(2, 10.0, 0.0));
    let sample = track.sample(150, 100).expect("sample");
    assert_eq!(sample.motion.map(|motion| motion.index), Some(1));
}

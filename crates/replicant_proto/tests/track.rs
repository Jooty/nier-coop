use replicant_proto::{MotionState, PartnerActivity, PlayerState, StateTrack, WeaponState};

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
            name_hash: 0,
        }),
        hp: 10,
        max_hp: 10,
        weapon: None,
        activity: Default::default(),
        mp: 0,
        max_mp: 0,
        level: 0,
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

fn playing(tick: u32, index: u16, frame: f32) -> PlayerState {
    PlayerState {
        motion: Some(MotionState {
            index,
            frame,
            speed: 1.0,
            looping: true,
            name_hash: 0,
        }),
        ..state(tick, 0.0, 0.0)
    }
}

#[test]
fn the_frame_of_one_motion_is_interpolated() {
    let mut track = StateTrack::new();
    track.push(0, playing(1, 497, 10.0));
    track.push(100, playing(2, 497, 16.0));
    let motion = track
        .sample(150, 100)
        .and_then(|s| s.motion)
        .expect("motion");
    assert_eq!(motion.index, 497);
    assert!((motion.frame - 13.0).abs() < 1e-4);
}

#[test]
fn the_earlier_motion_plays_on_until_the_new_one_is_due() {
    let mut track = StateTrack::new();
    track.push(0, playing(1, 497, 10.0));
    track.push(100, playing(2, 495, 2.0));
    let motion = track
        .sample(150, 100)
        .and_then(|s| s.motion)
        .expect("motion");
    assert_eq!(motion.index, 497);
    assert!((motion.frame - 13.0).abs() < 1e-4);
}

#[test]
fn a_wrapped_loop_plays_forward_at_its_speed_instead_of_backwards() {
    let mut track = StateTrack::new();
    track.push(0, playing(1, 497, 58.0));
    track.push(100, playing(2, 497, 3.0));
    let motion = track
        .sample(150, 100)
        .and_then(|s| s.motion)
        .expect("motion");
    assert!((motion.frame - 61.0).abs() < 1e-4);
}

#[test]
fn past_the_newest_state_the_motion_plays_on_for_a_while_then_holds() {
    let mut track = StateTrack::new();
    track.push(0, playing(1, 497, 10.0));
    let frame_at = |now| {
        track
            .sample(now, 0)
            .and_then(|s| s.motion)
            .expect("motion")
            .frame
    };
    assert!((frame_at(100) - 16.0).abs() < 1e-4);
    assert!((frame_at(5_000) - frame_at(600)).abs() < 1e-4);
}

#[test]
fn a_paused_partner_is_carried_by_the_sample() {
    let mut track = StateTrack::new();
    track.push(
        0,
        PlayerState {
            activity: PartnerActivity::InMenu,
            mp: 0,
            max_mp: 0,
            ..state(1, 0.0, 0.0)
        },
    );
    let sample = track.sample(10, 0).expect("sample");
    assert!(sample.activity.world_paused());
    assert_eq!(sample.activity.label(), Some("in menu"));
}

#[test]
fn the_weapon_rides_along_with_the_sample() {
    let weapon = WeaponState {
        id: 1,
        level: 0,
        in_hand: false,
        visible: true,
    };
    let mut track = StateTrack::new();
    track.push(
        0,
        PlayerState {
            weapon: Some(weapon),
            ..state(1, 0.0, 0.0)
        },
    );
    assert_eq!(track.sample(50, 0).and_then(|s| s.weapon), Some(weapon));
}

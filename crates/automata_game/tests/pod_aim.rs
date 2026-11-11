use automata_game::partner::{
    AimRayPoints, CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START,
    LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT, POD_SHOT_UPDATE_FUNCTION_START, PodAim,
    code_starts_with,
};

const A_DEGREE_IN_RADIANS: f32 = core::f32::consts::PI / 180.0;

fn aim() -> PodAim {
    PodAim {
        eye: [100.0, 12.0, -250.0],
        direction: [0.6, 0.0, 0.8],
    }
}

fn direction_between(from: [f32; 3], to: [f32; 3]) -> [f32; 3] {
    let along = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = (along[0] * along[0] + along[1] * along[1] + along[2] * along[2]).sqrt();
    along.map(|axis| axis / length)
}

fn angle_between(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos()
}

#[test]
fn the_aim_of_two_ray_points_starts_at_the_near_one_and_points_at_the_far_one() {
    let near = [10.0, 5.0, 20.0];
    let far = [10.0, 5.0, 5020.0];
    assert_eq!(
        PodAim::from_ray_points(near, far),
        Some(PodAim {
            eye: near,
            direction: [0.0, 0.0, 1.0],
        })
    );
}

#[test]
fn two_ray_points_that_give_no_direction_give_no_aim() {
    let point = [10.0, 5.0, 20.0];
    assert_eq!(PodAim::from_ray_points(point, point), None);
    assert_eq!(PodAim::from_ray_points(point, [f32::NAN, 5.0, 20.0]), None);
    assert_eq!(
        PodAim::from_ray_points([f32::INFINITY, 5.0, 20.0], point),
        None
    );
    assert_eq!(
        PodAim::from_ray_points([2_000_000.0, 0.0, 0.0], [2_000_100.0, 0.0, 0.0]),
        None
    );
}

#[test]
fn the_pod_shot_gets_the_eye_at_the_near_depth_and_a_point_along_the_aim_at_the_far_depth() {
    let aim = aim();
    assert_eq!(aim.ray_point(0.0), Some(aim.eye));
    let far = aim.ray_point(1.0).expect("a far point");
    let expected = [
        100.0 + 0.6 * LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT,
        12.0,
        -250.0 + 0.8 * LENGTH_OF_THE_AIM_RAY_GIVEN_TO_A_POD_SHOT,
    ];
    for axis in 0..3 {
        assert!((far[axis] - expected[axis]).abs() < 0.01);
    }
}

#[test]
fn the_direction_the_game_takes_from_the_two_points_is_the_senders_within_a_degree() {
    let senders = [
        [0.6, 0.0, 0.8],
        [0.0, 1.0, 0.0],
        [-0.577_350_3, -0.577_350_3, 0.577_350_3],
        [0.999_847_7, 0.017_452_4, 0.0],
    ];
    for direction in senders {
        let aim = PodAim {
            eye: [48_000.0, -900.0, 123_456.0],
            direction,
        };
        let near = aim.ray_point(0.0).expect("a near point");
        let far = aim.ray_point(1.0).expect("a far point");
        let taken = direction_between(near, far);
        assert!(angle_between(taken, direction) < A_DEGREE_IN_RADIANS);
    }
}

#[test]
fn what_one_copy_records_gives_the_other_copy_the_same_ray() {
    let mut recorded = AimRayPoints::new();
    recorded.note(0.0, [3.0, 4.0, 5.0]);
    recorded.note(1.0, [3.0, 4.0, 9005.0]);
    let sent = recorded.aim().expect("an aim");
    let near = sent.ray_point(0.0).expect("a near point");
    let far = sent.ray_point(1.0).expect("a far point");
    assert_eq!(near, [3.0, 4.0, 5.0]);
    assert!(angle_between(direction_between(near, far), [0.0, 0.0, 1.0]) < A_DEGREE_IN_RADIANS);
}

#[test]
fn an_aim_that_makes_no_sense_gives_the_pod_shot_nothing() {
    let senseless = [
        PodAim {
            direction: [0.0, 0.0, 0.0],
            ..aim()
        },
        PodAim {
            direction: [f32::NAN, 0.0, 1.0],
            ..aim()
        },
        PodAim {
            direction: [0.0, 3.0, 4.0],
            ..aim()
        },
        PodAim {
            eye: [f32::INFINITY, 0.0, 0.0],
            ..aim()
        },
        PodAim {
            eye: [0.0, -2_000_000.0, 0.0],
            ..aim()
        },
    ];
    for aim in senseless {
        assert_eq!(aim.checked(), None);
        assert_eq!(aim.ray_point(0.0), None);
        assert_eq!(aim.ray_point(1.0), None);
    }
}

#[test]
fn a_screen_depth_the_pod_shot_never_asks_for_gets_nothing() {
    for depth in [-0.5, 1.5, f32::NAN, f32::INFINITY] {
        assert_eq!(aim().ray_point(depth), None);
    }
}

#[test]
fn a_direction_a_little_off_one_unit_after_the_wire_is_still_an_aim() {
    let rounded = PodAim {
        direction: [0.600_1, 0.0, 0.800_1],
        ..aim()
    };
    assert_eq!(rounded.checked(), Some(rounded));
}

#[test]
fn only_the_near_and_the_far_depth_are_recorded_and_both_are_needed() {
    let mut points = AimRayPoints::new();
    assert_eq!(points.aim(), None);
    points.note(0.5, [9.0, 9.0, 9.0]);
    assert_eq!(points.aim(), None);
    points.note(0.0, [1.0, 2.0, 3.0]);
    assert_eq!(points.aim(), None);
    points.note(1.0, [1.0, 102.0, 3.0]);
    assert_eq!(
        points.aim(),
        Some(PodAim {
            eye: [1.0, 2.0, 3.0],
            direction: [0.0, 1.0, 0.0],
        })
    );
}

#[test]
fn a_new_record_starts_with_no_points() {
    assert_eq!(AimRayPoints::new(), AimRayPoints::default());
}

#[test]
fn the_noted_function_starts_are_told_from_other_code() {
    assert!(code_starts_with(
        &POD_SHOT_UPDATE_FUNCTION_START,
        &POD_SHOT_UPDATE_FUNCTION_START
    ));
    assert!(code_starts_with(
        &CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START,
        &CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START
    ));
    let mut hooked_by_someone_else = CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START;
    hooked_by_someone_else[0] = 0xE9;
    assert!(!code_starts_with(
        &hooked_by_someone_else,
        &CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START
    ));
    assert!(!code_starts_with(
        &POD_SHOT_UPDATE_FUNCTION_START[..8],
        &POD_SHOT_UPDATE_FUNCTION_START
    ));
}

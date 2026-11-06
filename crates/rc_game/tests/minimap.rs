use rc_game::marker::CameraState;
use rc_game::minimap::{MinimapTuning, minimap_dot};

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];
const TOLERANCE: f32 = 1.0e-3;

fn camera_with(world_matrix: [f32; 16]) -> CameraState {
    CameraState {
        world_matrix,
        view_projection: IDENTITY,
        vertical_fov: 0.8,
        aspect: 16.0 / 9.0,
        near: 0.1,
        far: 1000.0,
        active_index: 0,
    }
}

fn tuning() -> MinimapTuning {
    MinimapTuning {
        rotate_with_camera: true,
        radius: 100.0,
        world_range: 50.0,
        ..MinimapTuning::new()
    }
}

#[test]
fn a_partner_straight_ahead_is_straight_up_on_the_minimap() {
    let dot = minimap_dot(
        [0.0; 3],
        [0.0, 0.0, 10.0],
        &camera_with(IDENTITY),
        &tuning(),
    )
    .unwrap();
    assert!(dot.offset_x.abs() < TOLERANCE);
    assert!((dot.offset_y + 20.0).abs() < TOLERANCE);
    assert!(!dot.clamped_to_edge);
}

#[test]
fn a_partner_to_the_camera_right_is_to_the_right_on_the_minimap() {
    let dot = minimap_dot(
        [0.0; 3],
        [-10.0, 0.0, 0.0],
        &camera_with(IDENTITY),
        &tuning(),
    )
    .unwrap();
    assert!((dot.offset_x - 20.0).abs() < TOLERANCE);
    assert!(dot.offset_y.abs() < TOLERANCE);
}

#[test]
fn the_dot_turns_with_the_camera() {
    let mut turned = IDENTITY;
    turned[0] = 0.0;
    turned[2] = 1.0;
    turned[8] = -1.0;
    turned[10] = 0.0;
    let dot = minimap_dot([0.0; 3], [10.0, 0.0, 0.0], &camera_with(turned), &tuning()).unwrap();
    assert!((dot.offset_y + 20.0).abs() < TOLERANCE);
    assert!(dot.offset_x.abs() < TOLERANCE);
}

#[test]
fn a_far_partner_sits_on_the_edge_pointing_at_them() {
    let dot = minimap_dot(
        [0.0; 3],
        [0.0, 0.0, 5000.0],
        &camera_with(IDENTITY),
        &tuning(),
    )
    .unwrap();
    assert!(dot.clamped_to_edge);
    assert!(dot.offset_x.abs() < TOLERANCE);
    assert!(dot.offset_y < -80.0 && dot.offset_y > -100.0);
}

#[test]
fn a_diagonal_partner_is_clamped_on_the_same_bearing() {
    let dot = minimap_dot(
        [0.0; 3],
        [-3000.0, 0.0, 3000.0],
        &camera_with(IDENTITY),
        &tuning(),
    )
    .unwrap();
    assert!(dot.clamped_to_edge);
    assert!((dot.offset_x + dot.offset_y).abs() < TOLERANCE);
    assert!((dot.offset_x.hypot(dot.offset_y) - 92.0).abs() < TOLERANCE);
}

#[test]
fn north_up_ignores_the_camera() {
    let tuning = MinimapTuning {
        rotate_with_camera: false,
        ..tuning()
    };
    let mut turned = IDENTITY;
    turned[0] = 0.0;
    turned[2] = 1.0;
    turned[8] = -1.0;
    turned[10] = 0.0;
    let dot = minimap_dot([0.0; 3], [0.0, 0.0, 10.0], &camera_with(turned), &tuning).unwrap();
    assert!((dot.offset_y + 20.0).abs() < TOLERANCE);
}

#[test]
fn non_finite_input_draws_nothing() {
    let camera = camera_with(IDENTITY);
    assert!(minimap_dot([0.0; 3], [f32::NAN, 0.0, 0.0], &camera, &tuning()).is_none());
    let broken = camera_with([0.0; 16]);
    assert!(minimap_dot([0.0; 3], [1.0, 0.0, 1.0], &broken, &tuning()).is_none());
}

#[test]
fn the_default_minimap_does_not_turn_with_the_camera() {
    assert!(!MinimapTuning::new().rotate_with_camera);
}

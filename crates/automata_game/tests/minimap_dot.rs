use automata_game::partner_hud::{MinimapTuning, TagCamera, minimap_dot};

// Row vectors: translation in the last row; the camera looks down minus z.
fn camera_at(eye: [f32; 3]) -> TagCamera {
    let mut view = [0.0_f32; 16];
    view[0] = 1.0;
    view[5] = 1.0;
    view[10] = 1.0;
    view[15] = 1.0;
    view[12] = -eye[0];
    view[13] = -eye[1];
    view[14] = -eye[2];
    let mut projection = [0.0_f32; 16];
    projection[0] = 1.0;
    projection[5] = 2.0;
    projection[10] = -1.0;
    projection[11] = -1.0;
    projection[14] = -0.1;
    TagCamera { view, projection }
}

// The same camera turned a quarter to its right: it looks down plus x.
fn camera_looking_down_plus_x() -> TagCamera {
    let mut camera = camera_at([0.0; 3]);
    camera.view = [0.0; 16];
    camera.view[2] = -1.0;
    camera.view[5] = 1.0;
    camera.view[8] = 1.0;
    camera.view[15] = 1.0;
    camera
}

#[test]
fn the_dot_is_up_when_the_partner_is_ahead_of_the_camera() {
    let tuning = MinimapTuning::new();
    let dot = minimap_dot([0.0; 3], [0.0, 0.0, -11.0], &camera_at([0.0; 3]), &tuning).unwrap();
    assert!(dot.offset_x.abs() < 1.0e-4);
    assert!((dot.offset_y + 11.8).abs() < 1.0e-3);
    assert!(!dot.clamped_to_edge);
}

#[test]
fn the_dot_turns_with_the_camera() {
    let tuning = MinimapTuning::new();
    let camera = camera_looking_down_plus_x();
    let ahead = minimap_dot([0.0; 3], [11.0, 0.0, 0.0], &camera, &tuning).unwrap();
    assert!(ahead.offset_x.abs() < 1.0e-4 && ahead.offset_y < 0.0);
    let to_the_right = minimap_dot([0.0; 3], [0.0, 0.0, 11.0], &camera, &tuning).unwrap();
    assert!(to_the_right.offset_x > 0.0 && to_the_right.offset_y.abs() < 1.0e-4);
}

#[test]
fn a_map_that_does_not_turn_keeps_minus_z_up() {
    let tuning = MinimapTuning {
        rotate_with_camera: false,
        ..MinimapTuning::new()
    };
    let camera = camera_looking_down_plus_x();
    let dot = minimap_dot([0.0; 3], [0.0, 0.0, -11.0], &camera, &tuning).unwrap();
    assert!(dot.offset_x.abs() < 1.0e-4 && dot.offset_y < 0.0);
}

#[test]
fn a_far_partner_stays_on_the_rim_in_their_direction() {
    let tuning = MinimapTuning::new();
    let dot = minimap_dot([0.0; 3], [9000.0, 0.0, 0.0], &camera_at([0.0; 3]), &tuning).unwrap();
    assert!(dot.clamped_to_edge);
    assert!((dot.offset_x - tuning.radius * 0.92).abs() < 1.0e-3);
    assert!(dot.offset_y.abs() < 1.0e-3);
}

#[test]
fn the_height_difference_does_not_move_the_dot() {
    let tuning = MinimapTuning::new();
    let camera = camera_at([0.0; 3]);
    assert_eq!(
        minimap_dot([0.0; 3], [3.0, 0.0, -4.0], &camera, &tuning),
        minimap_dot([0.0, 50.0, 0.0], [3.0, -20.0, -4.0], &camera, &tuning)
    );
}

#[test]
fn a_dot_needs_finite_positions_and_a_camera_that_is_not_looking_straight_down() {
    let tuning = MinimapTuning::new();
    let camera = camera_at([0.0; 3]);
    assert_eq!(
        minimap_dot([f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], &camera, &tuning),
        None
    );
    let mut straight_down = camera;
    straight_down.view = [0.0; 16];
    assert_eq!(
        minimap_dot([0.0; 3], [1.0, 0.0, 0.0], &straight_down, &tuning),
        None
    );
}

use rc_game::marker::{
    CAMERA_ACTIVE_INDEX_RVA, CAMERA_ASPECT_RVA, CAMERA_FAR_RVA, CAMERA_FOV_RVA, CAMERA_MATRIX_RVA,
    CAMERA_NEAR_RVA, CONTROL_MASK_FIRST_RVA, CameraState, EDGE_MARGIN_FRACTION,
    EVENT_SCENE_BYTE_RVA, MarkerVisibility, Projected, ProjectionMethod, VIEW_PROJECTION_RVA,
    fade_alpha, hidden_reason, marker_label, project, read_camera_state, read_world_flags,
};

use rc_proto::PartnerActivity;

use crate::fake_memory::FakeMemory;

const WIDTH: f32 = 1920.0;
const HEIGHT: f32 = 1080.0;
const PAPER_FOV: f32 = core::f32::consts::FRAC_PI_4;
const PAPER_ASPECT: f32 = 16.0 / 9.0;
const PIXEL_TOLERANCE: f32 = 1.0;

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn paper_camera() -> CameraState {
    let tan_half = (PAPER_FOV * 0.5).tan();
    let mut view_projection = [0.0_f32; 16];
    view_projection[0] = -1.0 / (tan_half * PAPER_ASPECT);
    view_projection[5] = 1.0 / tan_half;
    view_projection[14] = 1.0;
    CameraState {
        world_matrix: IDENTITY,
        view_projection,
        vertical_fov: PAPER_FOV,
        aspect: PAPER_ASPECT,
        near: 0.05,
        far: 2500.0,
        active_index: 0,
    }
}

fn both_methods() -> [ProjectionMethod; 2] {
    [
        ProjectionMethod::WorldMatrix,
        ProjectionMethod::ViewProjection,
    ]
}

fn assert_on_screen(projected: Projected, want_x: f32, want_y: f32) {
    let Projected::OnScreen { x, y } = projected else {
        panic!("expected on screen, got {projected:?}");
    };
    assert!((x - want_x).abs() <= PIXEL_TOLERANCE, "x {x} vs {want_x}");
    assert!((y - want_y).abs() <= PIXEL_TOLERANCE, "y {y} vs {want_y}");
}

#[test]
fn a_point_straight_ahead_lands_in_the_middle() {
    for method in both_methods() {
        let projected = project(&paper_camera(), method, [0.0, 0.0, 10.0], WIDTH, HEIGHT);
        assert_on_screen(projected, 960.0, 540.0);
    }
}

#[test]
fn a_point_to_the_left_of_the_camera_lands_right_of_centre_per_the_paper_table() {
    for method in both_methods() {
        let projected = project(&paper_camera(), method, [-2.0, 0.0, 10.0], WIDTH, HEIGHT);
        assert_on_screen(projected, 1220.7, 540.0);
    }
}

#[test]
fn a_point_above_lands_higher_on_screen() {
    for method in both_methods() {
        let projected = project(&paper_camera(), method, [0.0, 2.0, 10.0], WIDTH, HEIGHT);
        assert_on_screen(projected, 960.0, 279.3);
    }
}

#[test]
fn a_point_behind_the_camera_is_an_arrow_not_a_tag() {
    for method in both_methods() {
        let projected = project(&paper_camera(), method, [0.0, 0.0, -5.0], WIDTH, HEIGHT);
        let Projected::EdgeArrow { x, y, .. } = projected else {
            panic!("expected arrow, got {projected:?}");
        };
        assert_eq!(x, 960.0);
        assert!(y > 540.0, "directly behind points down, got y {y}");
    }
}

#[test]
fn behind_and_on_the_cameras_left_turns_the_arrow_left() {
    for method in both_methods() {
        let projected = project(&paper_camera(), method, [3.0, 0.0, -5.0], WIDTH, HEIGHT);
        let Projected::EdgeArrow { x, .. } = projected else {
            panic!("expected arrow, got {projected:?}");
        };
        assert!(
            x < 960.0,
            "world +x is the camera's left in the paper frame"
        );
    }
}

#[test]
fn an_off_screen_point_in_front_keeps_its_direction_on_the_edge() {
    let projected = project(
        &paper_camera(),
        ProjectionMethod::WorldMatrix,
        [30.0, 0.0, 10.0],
        WIDTH,
        HEIGHT,
    );
    let Projected::EdgeArrow {
        x,
        y,
        angle_radians,
    } = projected
    else {
        panic!("expected arrow, got {projected:?}");
    };
    assert!(x < 960.0);
    assert_eq!(y, 540.0);
    assert!((angle_radians.abs() - core::f32::consts::PI).abs() < 1.0e-4);
}

#[test]
fn the_arrow_stays_inside_the_margin_for_sixteen_directions() {
    let camera = paper_camera();
    let margin_x = EDGE_MARGIN_FRACTION * WIDTH * 0.5;
    let margin_y = EDGE_MARGIN_FRACTION * HEIGHT * 0.5;
    for step in 0..16 {
        let heading = step as f32 * core::f32::consts::TAU / 16.0;
        let point = [heading.cos() * 500.0, heading.sin() * 500.0, 10.0];
        let behind = [heading.cos() * 500.0, heading.sin() * 500.0, -10.0];
        for point in [point, behind] {
            let projected = project(&camera, ProjectionMethod::WorldMatrix, point, WIDTH, HEIGHT);
            let Projected::EdgeArrow { x, y, .. } = projected else {
                panic!("direction {step}: expected arrow, got {projected:?}");
            };
            assert!(x >= margin_x - 1.0 && x <= WIDTH - margin_x + 1.0, "x {x}");
            assert!(y >= margin_y - 1.0 && y <= HEIGHT - margin_y + 1.0, "y {y}");
            let on_edge = (x - margin_x).abs() <= 1.0
                || (x - (WIDTH - margin_x)).abs() <= 1.0
                || (y - margin_y).abs() <= 1.0
                || (y - (HEIGHT - margin_y)).abs() <= 1.0;
            assert!(
                on_edge,
                "direction {step} is not on the rectangle: {x}, {y}"
            );
        }
    }
}

#[test]
fn unusable_inputs_draw_nothing() {
    let mut camera = paper_camera();
    let point = [0.0, 0.0, 10.0];
    let method = ProjectionMethod::WorldMatrix;
    assert_eq!(
        project(&camera, method, point, 0.0, HEIGHT),
        Projected::Invalid
    );
    assert_eq!(
        project(&camera, method, point, WIDTH, 0.0),
        Projected::Invalid
    );
    assert_eq!(
        project(&camera, method, [f32::NAN, 0.0, 1.0], WIDTH, HEIGHT),
        Projected::Invalid
    );
    camera.aspect = 0.0;
    assert_eq!(
        project(&camera, method, point, WIDTH, HEIGHT),
        Projected::Invalid
    );
    camera.aspect = PAPER_ASPECT;
    camera.vertical_fov = 3.5;
    assert_eq!(
        project(&camera, method, point, WIDTH, HEIGHT),
        Projected::Invalid
    );
    camera.vertical_fov = 0.01;
    assert_eq!(
        project(&camera, method, point, WIDTH, HEIGHT),
        Projected::Invalid
    );
}

fn floats_to_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[test]
fn the_camera_is_read_from_the_manager_addresses() {
    let mut memory = FakeMemory::new();
    let camera = paper_camera();
    memory.put_at_rva(CAMERA_MATRIX_RVA, &floats_to_bytes(&camera.world_matrix));
    memory.put_at_rva(
        VIEW_PROJECTION_RVA,
        &floats_to_bytes(&camera.view_projection),
    );
    memory.put_at_rva(CAMERA_FOV_RVA, &camera.vertical_fov.to_le_bytes());
    memory.put_at_rva(CAMERA_ASPECT_RVA, &camera.aspect.to_le_bytes());
    memory.put_at_rva(CAMERA_NEAR_RVA, &camera.near.to_le_bytes());
    memory.put_at_rva(CAMERA_FAR_RVA, &camera.far.to_le_bytes());
    memory.put_at_rva(CAMERA_ACTIVE_INDEX_RVA, &0_i32.to_le_bytes());
    assert_eq!(read_camera_state(&memory).unwrap(), Some(camera));
}

#[test]
fn an_unreadable_camera_is_an_error_not_a_guess() {
    assert!(read_camera_state(&FakeMemory::new()).is_err());
}

fn visible() -> MarkerVisibility {
    MarkerVisibility {
        enabled: true,
        peer_known: true,
        same_phase: true,
        peer_state_age_ms: 100,
        camera: Some(paper_camera()),
        event_scene: false,
        world_frozen: false,
    }
}

#[test]
fn the_marker_shows_for_a_fresh_partner_in_the_same_phase() {
    assert_eq!(hidden_reason(&visible()), None);
}

#[test]
fn the_marker_hides_with_a_reason() {
    let cases: [(&str, fn(&mut MarkerVisibility)); 8] = [
        ("turned off", |v| v.enabled = false),
        ("no partner state", |v| v.peer_known = false),
        ("partner in another phase", |v| v.same_phase = false),
        ("partner state is stale", |v| v.peer_state_age_ms = 10_000),
        ("cutscene running", |v| v.event_scene = true),
        ("game paused or in a menu", |v| v.world_frozen = true),
        ("camera unreadable", |v| v.camera = None),
        ("camera not ready", |v| {
            v.camera.as_mut().unwrap().active_index = -1;
        }),
    ];
    for (reason, change) in cases {
        let mut visibility = visible();
        change(&mut visibility);
        assert_eq!(hidden_reason(&visibility), Some(reason));
    }
}

fn memory_with_flags(event: u8, mask: [u32; 4]) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(EVENT_SCENE_BYTE_RVA, &[event]);
    let bytes: Vec<u8> = mask.iter().flat_map(|slot| slot.to_le_bytes()).collect();
    memory.put_at_rva(CONTROL_MASK_FIRST_RVA, &bytes);
    memory
}

#[test]
fn the_event_byte_means_a_cutscene() {
    let flags = read_world_flags(&memory_with_flags(1, [0; 4])).unwrap();
    assert!(flags.event_scene);
    assert!(!flags.world_frozen());
}

#[test]
fn the_world_is_frozen_when_any_mask_slot_has_bit_two() {
    for slot in 0..4 {
        let mut mask = [0_u32; 4];
        mask[slot] = 0x2;
        let flags = read_world_flags(&memory_with_flags(0, mask)).unwrap();
        assert!(flags.world_frozen(), "slot {slot}");
    }
    let other_bits = read_world_flags(&memory_with_flags(0, [0x1, 0x4, 0x8, 0x0])).unwrap();
    assert!(!other_bits.world_frozen());
}

#[test]
fn the_flags_say_what_the_partner_is_doing_and_a_cutscene_wins() {
    let activity = |event, mask| {
        read_world_flags(&memory_with_flags(event, mask))
            .unwrap()
            .activity()
    };
    assert_eq!(activity(0, [0; 4]), PartnerActivity::Playing);
    assert_eq!(activity(0, [0, 0x19F, 0, 0]), PartnerActivity::InMenu);
    assert_eq!(activity(1, [0x2, 0, 0, 0]), PartnerActivity::InCutscene);
    assert_eq!(activity(0, [0x20, 0, 0, 0]), PartnerActivity::Playing);
}

#[test]
fn the_tag_fades_with_distance_and_never_vanishes() {
    assert_eq!(fade_alpha(0.0), 1.0);
    assert!(fade_alpha(120.0) < 1.0 && fade_alpha(120.0) > fade_alpha(1000.0));
    assert!(fade_alpha(1.0e6) > 0.0);
}

#[test]
fn label_shows_name_distance_and_ping() {
    assert_eq!(
        marker_label("Nier", 12.4, Some(45)),
        "Nier \u{b7} 12 m \u{b7} 45 ms"
    );
}

#[test]
fn label_hides_the_ping_when_unknown() {
    assert_eq!(marker_label("Nier", 12.4, None), "Nier \u{b7} 12 m");
}

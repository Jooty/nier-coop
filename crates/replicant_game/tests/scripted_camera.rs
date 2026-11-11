use replicant_game::fight_sync::{
    CAMERA_SETTLE_MS, GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS, GuestCameraFollow, GuestCameraStep,
    GuestCameraView, HostCameraSampler, ScriptedCameraNative, angle_between_degrees,
    forward_of_world_matrix, look_angles_degrees, read_native_numbers,
};
use replicant_game::story_jump::{
    NATIVE_ARG_FIRST_OFFSET_BYTES, NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG,
    NATIVE_ARG_STRIDE_BYTES, NATIVE_ARG_VALUE_OFFSET_BYTES, NATIVE_ARGS_HEADER_BYTES,
};
use replicant_proto::FightCameraAngle;

use crate::fake_memory::FakeMemory;

const ARGS: u64 = 0x5_0000;

fn host_angle() -> FightCameraAngle {
    FightCameraAngle {
        phase: 16,
        native: ScriptedCameraNative::SetFreeCameraDirectionFixed.code(),
        forward: [1.0, 0.0, 0.0],
        active_camera: 0,
    }
}

fn view(forward: [f32; 3], menu_open: bool) -> GuestCameraView {
    GuestCameraView {
        menu_open,
        in_event_scene: false,
        phase: Some(16),
        forward: Some(forward),
        active_camera: 0,
    }
}

#[test]
fn every_camera_native_keeps_its_wire_code() {
    for native in ScriptedCameraNative::ALL {
        assert_eq!(ScriptedCameraNative::from_code(native.code()), Some(native));
    }
    assert_eq!(ScriptedCameraNative::from_code(0), None);
}

#[test]
fn the_host_reads_the_angle_once_the_camera_settled_and_waits_out_a_pause_menu() {
    let mut sampler = HostCameraSampler::default();
    sampler.note_call(ScriptedCameraNative::SetFreeCameraDistanceOffset, 0);
    assert_eq!(sampler.poll(CAMERA_SETTLE_MS, false), None);
    sampler.note_call(ScriptedCameraNative::SetFreeCameraDirectionFixed, 0);
    assert_eq!(sampler.poll(CAMERA_SETTLE_MS - 1, false), None);
    assert_eq!(sampler.poll(CAMERA_SETTLE_MS, true), None);
    assert_eq!(sampler.poll(2 * CAMERA_SETTLE_MS - 1, false), None);
    assert_eq!(
        sampler.poll(2 * CAMERA_SETTLE_MS, false),
        Some(ScriptedCameraNative::SetFreeCameraDirectionFixed)
    );
    assert_eq!(sampler.poll(10 * CAMERA_SETTLE_MS, false), None);
}

#[test]
fn the_guest_sets_the_hosts_angle_only_after_its_menu_closes_and_checks_it_after() {
    let mut follow = GuestCameraFollow::default();
    follow.note_own_call(0);
    follow.receive(host_angle(), 0);
    let late = CAMERA_SETTLE_MS + GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS;
    assert_eq!(follow.poll(late, &view([0.0, 0.0, 1.0], true)), None);
    let Some(GuestCameraStep::Apply { gap_degrees, .. }) =
        follow.poll(late, &view([0.0, 0.0, 1.0], false))
    else {
        panic!("the host's angle was not applied");
    };
    assert!((gap_degrees - 90.0).abs() < 0.01);
    assert_eq!(follow.poll(late + 1, &view([1.0, 0.0, 0.0], false)), None);
    let verify = follow.poll(late + CAMERA_SETTLE_MS, &view([1.0, 0.0, 0.0], false));
    assert!(
        matches!(verify, Some(GuestCameraStep::Verify { gap_degrees, .. }) if gap_degrees < 0.01)
    );
}

#[test]
fn the_guest_leaves_a_matching_camera_alone_and_drops_an_angle_of_another_phase() {
    let mut follow = GuestCameraFollow::default();
    follow.receive(host_angle(), 0);
    let at = GUEST_CAMERA_WAIT_AFTER_RECEIPT_MS;
    assert!(matches!(
        follow.poll(at, &view([0.99, 0.05, 0.0], false)),
        Some(GuestCameraStep::AlreadyMatches { .. })
    ));
    follow.receive(host_angle(), 0);
    let mut elsewhere = view([0.0, 0.0, 1.0], false);
    elsewhere.phase = Some(17);
    assert_eq!(
        follow.poll(at, &elsewhere),
        Some(GuestCameraStep::Expired(host_angle()))
    );
}

#[test]
fn the_look_direction_is_column_two_of_the_world_matrix() {
    let mut matrix = [0.0_f32; 16];
    matrix[2] = 3.0;
    matrix[6] = -3.0;
    let forward = forward_of_world_matrix(&matrix).unwrap();
    let (yaw, pitch) = look_angles_degrees(forward);
    assert!((yaw - 90.0).abs() < 0.01 && (pitch + 45.0).abs() < 0.01);
    assert_eq!(forward_of_world_matrix(&[0.0; 16]), None);
    assert!((angle_between_degrees([1.0, 0.0, 0.0], [-2.0, 0.0, 0.0]) - 180.0).abs() < 0.01);
}

#[test]
fn a_natives_numbers_are_read_from_real_and_integer_slots() {
    let mut memory = FakeMemory::new();
    let slot = |index: usize| {
        ARGS + (NATIVE_ARGS_HEADER_BYTES
            + NATIVE_ARG_FIRST_OFFSET_BYTES
            + index * NATIVE_ARG_STRIDE_BYTES) as u64
    };
    let value = NATIVE_ARG_VALUE_OFFSET_BYTES as u64;
    memory.put(slot(0), &NATIVE_ARG_REAL_TYPE_TAG.to_le_bytes());
    memory.put(slot(0) + value, &(-11.5_f64).to_le_bytes());
    memory.put(slot(1), &NATIVE_ARG_INTEGER_TYPE_TAG.to_le_bytes());
    memory.put(slot(1) + value, &3_i32.to_le_bytes());
    memory.put(slot(2), &9_u32.to_le_bytes());
    assert_eq!(
        read_native_numbers(&memory, ARGS, 4),
        vec![Some(-11.5), Some(3.0), None, None]
    );
}

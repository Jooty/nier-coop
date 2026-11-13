use replicant_proto::{
    FightCamera, FightCameraPose, MAX_FIGHT_CAMERA_INDEX, Message, PROTOCOL_VERSION, WireError,
    decode, encode,
};

fn a_switch_to_the_fixed_camera() -> FightCamera {
    FightCamera {
        phase: 16,
        seq: 7,
        natives_before: 3,
        camera: 8,
        blend_seconds: 3.0,
        blend_kind: 1,
        pose: Some(FightCameraPose {
            eye: [-0.054, 74.72, 5.197],
            look_at: [-0.017, 49.389, -65.29],
        }),
    }
}

#[test]
fn the_protocol_number_went_up_with_the_fight_camera() {
    assert_eq!(PROTOCOL_VERSION, 61);
}

#[test]
fn a_fight_camera_round_trips_and_is_relayed() {
    let back_to_snow = FightCamera {
        camera: 0,
        pose: None,
        ..a_switch_to_the_fixed_camera()
    };
    for camera in [a_switch_to_the_fixed_camera(), back_to_snow] {
        let message = Message::FightCamera(camera);
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert_eq!(message.validate(), Ok(()));
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
    assert_eq!(
        Message::FightCamera(a_switch_to_the_fixed_camera()).kind_name(),
        "FightCamera"
    );
}

#[test]
fn a_fight_camera_out_of_range_is_refused() {
    let past = FightCamera {
        camera: MAX_FIGHT_CAMERA_INDEX + 1,
        ..a_switch_to_the_fixed_camera()
    };
    let backwards = FightCamera {
        blend_seconds: -1.0,
        ..a_switch_to_the_fixed_camera()
    };
    let nowhere = FightCamera {
        pose: Some(FightCameraPose {
            eye: [0.0, f32::NAN, 0.0],
            look_at: [0.0; 3],
        }),
        ..a_switch_to_the_fixed_camera()
    };
    assert_eq!(
        Message::FightCamera(past).validate(),
        Err(WireError::OutOfRange { field: "camera" })
    );
    assert_eq!(
        Message::FightCamera(backwards).validate(),
        Err(WireError::OutOfRange {
            field: "blend_seconds"
        })
    );
    assert_eq!(
        Message::FightCamera(nowhere).validate(),
        Err(WireError::OutOfRange { field: "pose" })
    );
}

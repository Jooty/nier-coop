use replicant_proto::{
    MAX_POSITION_ABS, MAX_POWER_MODEL_INSTANCE, Message, MotionState, PowerModelEnded,
    PowerModelKind, PowerModelMotion, PowerModelState, WireError, decode, encode,
};

fn state() -> PowerModelState {
    PowerModelState {
        kind: PowerModelKind::BlackHand030,
        instance: 12,
        position: [1.0, 2.0, 3.0],
        yaw_radians: 0.5,
    }
}

#[test]
fn a_power_model_round_trips_and_is_relayed() {
    let message = Message::PowerModel(state());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_power_model_at_a_bad_position_yaw_or_instance_is_refused() {
    let mut far = state();
    far.position[2] = MAX_POSITION_ABS + 1.0;
    let mut spinning = state();
    spinning.yaw_radians = f32::NAN;
    let mut numerous = state();
    numerous.instance = MAX_POWER_MODEL_INSTANCE + 1;
    for (bad, field) in [
        (far, "power model position"),
        (spinning, "power model yaw"),
        (numerous, "power model instance"),
    ] {
        assert_eq!(bad.validate(), Err(WireError::OutOfRange { field }));
    }
}

fn motion() -> PowerModelMotion {
    PowerModelMotion {
        instance: 12,
        motion: MotionState {
            index: 3,
            frame: 10.5,
            speed: 1.0,
            looping: false,
            name_hash: 0,
        },
    }
}

#[test]
fn a_power_motion_and_its_end_round_trip_and_are_relayed() {
    for message in [
        Message::PowerMotion(motion()),
        Message::PowerEnded(PowerModelEnded {
            instance: 12,
            lasted_ms: 900,
        }),
    ] {
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_power_motion_with_a_bad_frame_or_instance_is_refused() {
    let mut late = motion();
    late.motion.frame = f32::NAN;
    let mut numerous = motion();
    numerous.instance = MAX_POWER_MODEL_INSTANCE + 1;
    assert_eq!(
        late.validate(),
        Err(WireError::OutOfRange {
            field: "motion_frame"
        })
    );
    assert_eq!(
        numerous.validate(),
        Err(WireError::OutOfRange {
            field: "power model instance"
        })
    );
    let ended = PowerModelEnded {
        instance: MAX_POWER_MODEL_INSTANCE + 1,
        lasted_ms: 0,
    };
    assert_eq!(
        ended.validate(),
        Err(WireError::OutOfRange {
            field: "power model instance"
        })
    );
}

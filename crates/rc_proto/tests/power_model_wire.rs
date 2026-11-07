use rc_proto::{
    MAX_POSITION_ABS, MAX_POWER_MODEL_INSTANCE, Message, PowerModelKind, PowerModelState,
    WireError, decode, encode,
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

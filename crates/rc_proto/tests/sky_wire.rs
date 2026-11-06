use rc_proto::{MAX_SKY_AREA, Message, SkyState, WireError, decode, encode};

fn sky() -> SkyState {
    SkyState {
        phase: 4,
        env_area: 12,
        env_blend_seconds: 1.5,
        wind_area: 2,
        weather: 2,
        weather_offset: 1,
    }
}

#[test]
fn a_sky_message_survives_a_round_trip_and_is_relayed() {
    let message = Message::Sky(sky());
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

#[test]
fn an_area_past_the_table_is_refused() {
    let message = Message::Sky(SkyState {
        env_area: MAX_SKY_AREA + 1,
        ..sky()
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "env_area" })
    );
}

#[test]
fn a_nan_blend_is_refused() {
    let message = Message::Sky(SkyState {
        env_blend_seconds: f32::NAN,
        ..sky()
    });
    assert!(message.validate().is_err());
}

#[test]
fn an_unset_wind_area_is_allowed() {
    let message = Message::Sky(SkyState {
        wind_area: -1,
        ..sky()
    });
    assert_eq!(message.validate(), Ok(()));
}

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

#[test]
fn a_record_chunk_round_trips_stays_under_the_cap_and_is_relayed() {
    let message = Message::SkyRecord(rc_proto::SkyRecordChunk {
        phase: 1,
        area: 30,
        offset: 384,
        bytes: vec![7; rc_proto::MAX_SKY_RECORD_CHUNK_BYTES],
    });
    let bytes = encode(&message).expect("valid chunk encodes");
    assert!(bytes.len() <= rc_proto::MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

#[test]
fn a_record_chunk_past_the_record_end_is_refused() {
    let message = Message::SkyRecord(rc_proto::SkyRecordChunk {
        phase: 1,
        area: 30,
        offset: 0xAA0 - 4,
        bytes: vec![0; 8],
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "offset" })
    );
}

fn assets(names: Vec<String>) -> Message {
    Message::SkyAssets(rc_proto::SkyAssets {
        phase: 1,
        area: 30,
        names,
    })
}

#[test]
fn sky_assets_round_trip_under_the_cap_and_are_relayed() {
    let message = assets(vec!["x".repeat(rc_proto::MAX_SKY_ASSET_NAME_BYTES); 5]);
    let bytes = encode(&message).expect("valid assets encode");
    assert!(bytes.len() <= rc_proto::MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

#[test]
fn sky_assets_need_five_short_clean_names() {
    assert!(assets(vec![String::new(); 4]).validate().is_err());
    assert!(assets(vec!["a".repeat(81); 5]).validate().is_err());
    assert!(assets(vec!["a\nb".to_owned(); 5]).validate().is_err());
}

use rc_proto::{
    CallbackFired, MAX_MESSAGE_BYTES, Message, PROTOCOL_VERSION, WireError, decode, encode,
};

fn fired() -> CallbackFired {
    CallbackFired {
        phase: 25,
        name: "Boss_Battle_Start".to_owned(),
    }
}

#[test]
fn a_callback_fired_round_trips_fits_a_message_and_is_relayed() {
    let message = Message::CallbackFired(fired());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
    assert_eq!(PROTOCOL_VERSION, 13);
}

#[test]
fn a_callback_fired_with_a_bad_name_is_refused() {
    for name in [String::new(), "two words".to_owned(), "x".repeat(64)] {
        let bad = CallbackFired { name, ..fired() };
        assert_eq!(
            bad.validate(),
            Err(WireError::OutOfRange {
                field: "callback fired name"
            })
        );
    }
}

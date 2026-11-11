use replicant_proto::{MAX_MESSAGE_BYTES, Message, MessageHandlerRan, WireError, decode, encode};

fn ran() -> MessageHandlerRan {
    MessageHandlerRan {
        phase: 25,
        name: "Talk_Text_01_GUARD_CHANGE".to_owned(),
    }
}

#[test]
fn a_message_handler_ran_round_trips_fits_a_message_and_is_relayed() {
    let message = Message::MessageHandlerRan(ran());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_message_handler_ran_with_a_bad_name_is_refused() {
    for name in [String::new(), "two words".to_owned(), "x".repeat(64)] {
        let bad = MessageHandlerRan { name, ..ran() };
        assert_eq!(
            bad.validate(),
            Err(WireError::OutOfRange {
                field: "message handler name"
            })
        );
    }
}

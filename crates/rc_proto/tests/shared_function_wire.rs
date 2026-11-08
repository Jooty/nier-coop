use rc_proto::{MAX_MESSAGE_BYTES, Message, SharedFunctionRan, WireError, decode, encode};

fn ran() -> SharedFunctionRan {
    SharedFunctionRan {
        phase: 25,
        name: "EID_0200_a0030g_End".to_owned(),
    }
}

#[test]
fn a_shared_function_ran_round_trips_fits_a_message_and_is_relayed() {
    let message = Message::SharedFunctionRan(ran());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_shared_function_ran_with_a_bad_name_is_refused() {
    for name in [String::new(), "two words".to_owned(), "x".repeat(64)] {
        let bad = SharedFunctionRan { name, ..ran() };
        assert_eq!(
            bad.validate(),
            Err(WireError::OutOfRange {
                field: "shared function name"
            })
        );
    }
}

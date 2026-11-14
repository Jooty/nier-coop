use replicant_proto::{MAX_MESSAGE_BYTES, Message, RuleRoomBreak, WireError, decode, encode};

#[test]
fn the_guests_rule_break_round_trips_is_relayed_and_leaves_the_protocol_number() {
    let message = Message::RuleRoomBreak(RuleRoomBreak {
        phase: 5,
        handler: "Jump_has_Done".to_owned(),
    });
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "RuleRoomBreak");
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
}

#[test]
fn a_rule_break_without_a_handler_name_is_refused() {
    let message = Message::RuleRoomBreak(RuleRoomBreak {
        phase: 5,
        handler: String::new(),
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange {
            field: "rule room break handler"
        })
    );
}

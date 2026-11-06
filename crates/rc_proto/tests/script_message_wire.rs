use rc_proto::{MAX_SCRIPT_MESSAGE_NUMBER, Message, ScriptMessage, WireError, decode, encode};

#[test]
fn a_script_message_round_trips_and_is_relayed() {
    let message = Message::ScriptMessage(ScriptMessage {
        phase: 7,
        message: 243,
        seq: 5,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_script_message_past_the_bit_mask_is_refused() {
    let message = Message::ScriptMessage(ScriptMessage {
        phase: 7,
        message: MAX_SCRIPT_MESSAGE_NUMBER + 1,
        seq: 1,
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "message" })
    );
}

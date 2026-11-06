use rc_proto::{
    MAX_MESSAGE_BYTES, MAX_STORY_CALL_ARGS, MAX_STORY_CALL_NAME_BYTES, MAX_STORY_CALL_TEXT_BYTES,
    Message, StoryCall, StoryCallArg, WireError, decode, encode,
};

fn call() -> StoryCall {
    StoryCall {
        phase: 12,
        pack: 0x1234_5678_9ABC_DEF0,
        function: "SomeEvent_Start".to_owned(),
        args: vec![
            StoryCallArg::Int(-3),
            StoryCallArg::Number(0x4010_0000_0000_0000),
            StoryCallArg::Text(b"point_a".to_vec()),
        ],
    }
}

#[test]
fn a_story_call_round_trips_in_both_messages_and_is_relayed() {
    for message in [Message::StoryTrigger(call()), Message::StoryRun(call())] {
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn the_largest_allowed_story_call_fits_in_one_message() {
    let biggest = StoryCall {
        phase: u16::MAX,
        pack: u64::MAX,
        function: "a".repeat(MAX_STORY_CALL_NAME_BYTES),
        args: vec![StoryCallArg::Text(vec![b'z'; MAX_STORY_CALL_TEXT_BYTES]); MAX_STORY_CALL_ARGS],
    };
    let bytes = encode(&Message::StoryRun(biggest)).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn a_story_call_outside_the_limits_is_refused() {
    let with = |change: fn(&mut StoryCall)| {
        let mut value = call();
        change(&mut value);
        Message::StoryTrigger(value).validate()
    };
    assert_eq!(
        with(|value| value.function.clear()),
        Err(WireError::OutOfRange { field: "function" })
    );
    assert_eq!(
        with(|value| value.function = "a".repeat(MAX_STORY_CALL_NAME_BYTES + 1)),
        Err(WireError::OutOfRange { field: "function" })
    );
    assert_eq!(
        with(|value| value.function = "has space".to_owned()),
        Err(WireError::OutOfRange { field: "function" })
    );
    assert_eq!(
        with(|value| value.args = vec![StoryCallArg::Int(1); MAX_STORY_CALL_ARGS + 1]),
        Err(WireError::OutOfRange { field: "args" })
    );
    assert_eq!(
        with(|value| value.args = vec![StoryCallArg::Text(vec![
            b'x';
            MAX_STORY_CALL_TEXT_BYTES + 1
        ])]),
        Err(WireError::OutOfRange { field: "text" })
    );
    assert_eq!(
        with(|value| value.args = vec![StoryCallArg::Text(vec![b'x', 0])]),
        Err(WireError::OutOfRange { field: "text" })
    );
}

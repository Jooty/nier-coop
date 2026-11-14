use replicant_proto::{
    MAX_MESSAGE_BYTES, Message, PROTOCOL_VERSION, SkipPromptAnswer, SkipPromptVerdict, decode,
    encode,
};

#[test]
fn a_skip_prompt_answer_round_trips_is_relayed_and_leaves_the_protocol_number() {
    for skip in [true, false] {
        let message = Message::SkipPromptAnswer(SkipPromptAnswer {
            phase: 7,
            prompt: 2437,
            skip,
        });
        assert!(message.is_relayed());
        assert_eq!(message.kind_name(), "SkipPromptAnswer");
        assert_eq!(message.validate(), Ok(()));
        let bytes = encode(&message).unwrap();
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(decode(&bytes), Ok(message));
    }
    assert_eq!(PROTOCOL_VERSION, 70);
}

#[test]
fn the_hosts_skip_verdict_round_trips_and_is_relayed() {
    for skip in [true, false] {
        let message = Message::SkipPromptVerdict(SkipPromptVerdict {
            phase: 7,
            prompt: 2437,
            skip,
        });
        assert!(message.is_relayed());
        assert_eq!(message.kind_name(), "SkipPromptVerdict");
        assert_eq!(message.validate(), Ok(()));
        let bytes = encode(&message).unwrap();
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(decode(&bytes), Ok(message));
    }
}

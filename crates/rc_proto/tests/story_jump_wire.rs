use rc_proto::{Message, StoryJump, StoryJumpPoint, decode, encode};

#[test]
fn a_story_jump_round_trips_and_is_relayed() {
    let message = Message::StoryJump(StoryJump {
        point: StoryJumpPoint::AfterPrologue,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn the_popola_talk_jump_round_trips_and_is_relayed() {
    let message = Message::StoryJump(StoryJump {
        point: StoryJumpPoint::AtPopolaTalk,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

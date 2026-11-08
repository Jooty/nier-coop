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

#[test]
fn the_next_day_jump_round_trips_and_is_relayed() {
    let message = Message::StoryJump(StoryJump {
        point: StoryJumpPoint::BeforeNextDay,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_teleport_here_round_trips_and_is_relayed() {
    let message = Message::TeleportHere(rc_proto::TeleportHere {
        phase: 7,
        position: [1.5, 2.0, -3.25],
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_teleport_here_with_a_non_finite_position_is_refused() {
    let message = Message::TeleportHere(rc_proto::TeleportHere {
        phase: 7,
        position: [f32::NAN, 0.0, 0.0],
    });
    assert!(message.validate().is_err());
}

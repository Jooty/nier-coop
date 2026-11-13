use automata_game::load_gate::StoryPlace;
use automata_game::story_steps::{StepAnswer, StoryStepAnswer, StoryStepAsk, StoryStepDone};
use automata_proto::{Message, WireError, decode, encode};
use nc_wire::GameMessage;

const STORY_STEP_DONE_VARIANT_TAG: u8 = 36;
const STORY_STEP_ASK_VARIANT_TAG: u8 = 37;
const STORY_STEP_ANSWER_VARIANT_TAG: u8 = 38;

fn a_done() -> Message {
    Message::StoryStepDone(StoryStepDone {
        number: 2,
        id: 0x1234,
        value: -1,
        place: StoryPlace::new(0x100, "A"),
    })
}

fn an_ask() -> Message {
    Message::StoryStepAsk(StoryStepAsk {
        id: 0x1234,
        value: 3,
        place: StoryPlace::new(0x100, "A"),
    })
}

fn an_answer(answer: StepAnswer) -> Message {
    Message::StoryStepAnswer(StoryStepAnswer { id: 0x1234, answer })
}

#[test]
fn the_three_story_step_messages_come_after_every_existing_variant() {
    for (message, tag, name) in [
        (a_done(), STORY_STEP_DONE_VARIANT_TAG, "StoryStepDone"),
        (an_ask(), STORY_STEP_ASK_VARIANT_TAG, "StoryStepAsk"),
        (
            an_answer(StepAnswer::NotStarted),
            STORY_STEP_ANSWER_VARIANT_TAG,
            "StoryStepAnswer",
        ),
    ] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(bytes[0], tag);
        assert_eq!(message.kind_name(), name);
    }
}

#[test]
fn the_story_step_messages_round_trip() {
    for message in [
        a_done(),
        an_ask(),
        an_answer(StepAnswer::NotStarted),
        an_answer(StepAnswer::OtherPlace),
    ] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_story_step_messages_are_relayed_reliable_and_not_latest_wins() {
    for message in [a_done(), an_ask(), an_answer(StepAnswer::NotStarted)] {
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert!(!GameMessage::forwarded_to_other_guests(&message));
        assert_eq!(GameMessage::ordered_tick(&message), None);
    }
}

#[test]
fn a_told_done_needs_a_number_and_a_readable_place() {
    let unnumbered = Message::StoryStepDone(StoryStepDone {
        number: 0,
        id: 1,
        value: 0,
        place: StoryPlace::new(0x100, "A"),
    });
    assert_eq!(
        unnumbered.validate(),
        Err(WireError::OutOfRange {
            field: "story_step_done",
        })
    );
    let mut place = StoryPlace::new(0x100, "A");
    place.name_len = 0;
    let unreadable_ask = Message::StoryStepAsk(StoryStepAsk {
        id: 1,
        value: 0,
        place,
    });
    assert_eq!(
        unreadable_ask.validate(),
        Err(WireError::OutOfRange {
            field: "story_step_ask",
        })
    );
    assert_eq!(a_done().validate(), Ok(()));
    assert_eq!(an_ask().validate(), Ok(()));
}

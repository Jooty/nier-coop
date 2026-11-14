use automata_proto::{
    AskVerdict, Message, StoryAsk, StoryAskAnswer, StoryMove, StoryPlace, StoryRequest, WireError,
    decode, encode,
};
use nc_wire::GameMessage;

const STORY_MOVE_VARIANT_TAG: u8 = 27;
const STORY_ASK_VARIANT_TAG: u8 = 28;
const STORY_ASK_ANSWER_VARIANT_TAG: u8 = 29;

fn a_full_request() -> StoryRequest {
    StoryRequest::Full {
        id: 0x100,
        name: "A".to_owned(),
        params: [1, 2, 3, 4, 5, 6],
    }
}

fn a_sub_phase_request() -> StoryRequest {
    StoryRequest::SubPhase {
        id: 0x100,
        name: "B".to_owned(),
    }
}

fn a_move() -> Message {
    Message::StoryMove(StoryMove {
        number: 3,
        from: StoryPlace::new(0x100, "B"),
        request: a_full_request(),
    })
}

fn an_ask() -> Message {
    Message::StoryAsk(StoryAsk {
        number: 4,
        at: StoryPlace::new(0x100, "A"),
        request: a_sub_phase_request(),
    })
}

fn an_answer() -> Message {
    Message::StoryAskAnswer(StoryAskAnswer {
        number: 4,
        verdict: AskVerdict::RefusedBusyTooLong,
    })
}

#[test]
fn the_three_story_messages_come_after_every_existing_variant() {
    for (message, tag, name) in [
        (a_move(), STORY_MOVE_VARIANT_TAG, "StoryMove"),
        (an_ask(), STORY_ASK_VARIANT_TAG, "StoryAsk"),
        (an_answer(), STORY_ASK_ANSWER_VARIANT_TAG, "StoryAskAnswer"),
    ] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(bytes[0], tag);
        assert_eq!(message.kind_name(), name);
    }
}

#[test]
fn the_story_messages_round_trip() {
    for message in [a_move(), an_ask(), an_answer()] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn every_verdict_round_trips() {
    for verdict in [
        AskVerdict::Made,
        AskVerdict::AlreadyMade,
        AskVerdict::RefusedOtherPlace,
        AskVerdict::RefusedBusyTooLong,
        AskVerdict::RefusedGuestWatches,
    ] {
        let message = Message::StoryAskAnswer(StoryAskAnswer { number: 1, verdict });
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_story_messages_are_relayed_reliable_and_not_latest_wins() {
    for message in [a_move(), an_ask(), an_answer()] {
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert!(!GameMessage::forwarded_to_other_guests(&message));
    }
}

#[test]
fn a_name_of_thirty_one_bytes_is_valid_and_thirty_two_is_not() {
    let move_with = |name: String| {
        Message::StoryMove(StoryMove {
            number: 1,
            from: StoryPlace::new(0x100, "A"),
            request: StoryRequest::SubPhase { id: 0x100, name },
        })
    };
    assert_eq!(move_with("N".repeat(31)).validate(), Ok(()));
    assert_eq!(
        move_with("N".repeat(32)).validate(),
        Err(WireError::OutOfRange {
            field: "story_move"
        })
    );
}

#[test]
fn a_move_or_ask_that_could_never_be_told_is_refused() {
    let bad_requests = [
        StoryRequest::SubPhase {
            id: 0x100,
            name: String::new(),
        },
        StoryRequest::SubPhase {
            id: 0xF00,
            name: "A".to_owned(),
        },
        StoryRequest::Full {
            id: -1,
            name: "A".to_owned(),
            params: [0; 6],
        },
        StoryRequest::Full {
            id: 0x100,
            name: "a b".to_owned(),
            params: [0; 6],
        },
    ];
    for request in bad_requests {
        let story_move = Message::StoryMove(StoryMove {
            number: 1,
            from: StoryPlace::new(0x100, "A"),
            request: request.clone(),
        });
        let ask = Message::StoryAsk(StoryAsk {
            number: 1,
            at: StoryPlace::new(0x100, "A"),
            request,
        });
        assert!(story_move.validate().is_err());
        assert!(ask.validate().is_err());
    }
}

#[test]
fn a_full_request_without_a_name_is_valid_and_takes_the_first_sub_phase() {
    let message = Message::StoryAsk(StoryAsk {
        number: 1,
        at: StoryPlace::new(0x100, "A"),
        request: StoryRequest::Full {
            id: 0x100,
            name: String::new(),
            params: [0; 6],
        },
    });
    assert_eq!(message.validate(), Ok(()));
}

#[test]
fn number_zero_and_a_broken_place_are_refused() {
    let mut broken_place = StoryPlace::new(0x100, "A");
    broken_place.name_len = 40;
    for message in [
        Message::StoryMove(StoryMove {
            number: 0,
            from: StoryPlace::new(0x100, "A"),
            request: a_full_request(),
        }),
        Message::StoryAsk(StoryAsk {
            number: 1,
            at: broken_place,
            request: a_full_request(),
        }),
    ] {
        assert!(message.validate().is_err());
    }
}

#[test]
fn the_guest_watches_verdict_is_appended_after_every_existing_value() {
    let tag_of = |verdict| {
        let bytes = encode(&Message::StoryAskAnswer(StoryAskAnswer {
            number: 1,
            verdict,
        }))
        .expect("encodes");
        *bytes.last().expect("not empty")
    };
    assert_eq!(tag_of(AskVerdict::RefusedBusyTooLong), 3);
    assert_eq!(tag_of(AskVerdict::RefusedGuestWatches), 4);
}

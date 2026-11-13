use automata_proto::{
    EventAsk, EventAskWhat, EventId, Message, PlayerState, WireError, decode, encode,
};
use nc_wire::GameMessage;

const EVENT_STARTED_VARIANT_TAG: u8 = 30;
const EVENT_ENDED_VARIANT_TAG: u8 = 31;
const EVENT_ASK_VARIANT_TAG: u8 = 32;

fn an_event() -> EventId {
    EventId {
        kind: 0,
        id: 0x123,
        sub: -1,
    }
}

fn a_start() -> Message {
    Message::EventStarted { id: an_event() }
}

fn an_end() -> Message {
    Message::EventEnded {
        id: an_event(),
        skipped: true,
    }
}

fn an_ask(what: EventAskWhat) -> Message {
    Message::EventAsk(EventAsk {
        id: an_event(),
        what,
    })
}

fn a_state() -> PlayerState {
    let Ok(Message::State(state)) = decode(
        &encode(&Message::State(PlayerState::from_snapshot(
            &Default::default(),
            1,
        )))
        .expect("encodes"),
    ) else {
        panic!("expected a state message");
    };
    state
}

#[test]
fn the_three_event_messages_come_after_every_existing_variant() {
    for (message, tag, name) in [
        (a_start(), EVENT_STARTED_VARIANT_TAG, "EventStarted"),
        (an_end(), EVENT_ENDED_VARIANT_TAG, "EventEnded"),
        (
            an_ask(EventAskWhat::Start),
            EVENT_ASK_VARIANT_TAG,
            "EventAsk",
        ),
    ] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(bytes[0], tag);
        assert_eq!(message.kind_name(), name);
    }
}

#[test]
fn the_event_messages_round_trip() {
    for message in [
        a_start(),
        an_end(),
        Message::EventEnded {
            id: an_event(),
            skipped: false,
        },
        an_ask(EventAskWhat::Start),
        an_ask(EventAskWhat::Skip),
    ] {
        let bytes = encode(&message).expect("encodes");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_event_messages_are_relayed_reliable_and_not_latest_wins() {
    for message in [a_start(), an_end(), an_ask(EventAskWhat::Skip)] {
        assert!(message.is_relayed());
        assert!(!message.is_latest_wins());
        assert!(!GameMessage::forwarded_to_other_guests(&message));
    }
}

#[test]
fn an_event_of_an_unknown_kind_is_refused() {
    let unknown = EventId {
        kind: 3,
        ..an_event()
    };
    assert_eq!(
        Message::EventStarted { id: unknown }.validate(),
        Err(WireError::OutOfRange { field: "event" })
    );
    assert_eq!(
        Message::EventEnded {
            id: unknown,
            skipped: false
        }
        .validate(),
        Err(WireError::OutOfRange { field: "event" })
    );
    assert_eq!(
        Message::EventAsk(EventAsk {
            id: unknown,
            what: EventAskWhat::Start
        })
        .validate(),
        Err(WireError::OutOfRange { field: "event_ask" })
    );
    for kind in 0..=2 {
        let known = EventId { kind, ..an_event() };
        assert_eq!(Message::EventStarted { id: known }.validate(), Ok(()));
    }
}

#[test]
fn a_state_tells_the_senders_playing_event() {
    for playing_event in [None, Some(an_event())] {
        let sent = PlayerState {
            playing_event,
            ..a_state()
        };
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.playing_event, playing_event);
        assert_eq!(Message::State(decoded).validate(), Ok(()));
    }
}

#[test]
fn a_state_made_from_a_body_plays_no_event_yet() {
    assert_eq!(a_state().playing_event, None);
}

#[test]
fn a_state_with_an_event_of_an_unknown_kind_is_refused() {
    let broken = PlayerState {
        playing_event: Some(EventId {
            kind: -1,
            ..an_event()
        }),
        ..a_state()
    };
    assert_eq!(
        Message::State(broken).validate(),
        Err(WireError::OutOfRange {
            field: "playing_event"
        })
    );
}

#[test]
fn an_event_id_reads_and_writes_its_twelve_game_bytes() {
    let id = an_event();
    let bytes = id.to_bytes();
    assert_eq!(
        bytes,
        [0, 0, 0, 0, 0x23, 0x01, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
    );
    assert_eq!(EventId::of_bytes(&bytes), Some(id));
    assert_eq!(EventId::of_bytes(&bytes[..11]), None);
}

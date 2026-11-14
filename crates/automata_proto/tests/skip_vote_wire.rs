use automata_proto::{EventId, Message, PlayerState, WireError, decode, encode};
use nc_wire::GameMessage;

const SKIP_ASK_VARIANT_TAG: u8 = 34;

fn an_event() -> EventId {
    EventId {
        kind: 0,
        id: 0x321,
        sub: -1,
    }
}

fn a_state() -> PlayerState {
    PlayerState::from_snapshot(&Default::default(), 1)
}

#[test]
fn the_skip_ask_comes_after_every_existing_variant() {
    let message = Message::SkipAsk { id: an_event() };
    let bytes = encode(&message).expect("encodes");
    assert_eq!(bytes[0], SKIP_ASK_VARIANT_TAG);
    assert_eq!(message.kind_name(), "SkipAsk");
    assert_eq!(decode(&bytes).expect("decodes"), message);
}

#[test]
fn the_skip_ask_is_relayed_reliable_and_not_latest_wins() {
    let message = Message::SkipAsk { id: an_event() };
    assert!(message.is_relayed());
    assert!(!message.is_latest_wins());
    assert!(!GameMessage::forwarded_to_other_guests(&message));
}

#[test]
fn a_skip_ask_for_an_event_of_an_unknown_kind_is_refused() {
    let message = Message::SkipAsk {
        id: EventId {
            kind: 7,
            ..an_event()
        },
    };
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "event" })
    );
}

#[test]
fn a_state_tells_whether_its_sender_is_unattended() {
    assert!(!a_state().unattended);
    for unattended in [false, true] {
        let sent = PlayerState {
            unattended,
            ..a_state()
        };
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.unattended, unattended);
    }
}

const EVENT_PAUSE_VARIANT_TAG: u8 = 39;

#[test]
fn the_event_pause_comes_after_every_existing_variant() {
    for paused in [false, true] {
        let message = Message::EventPause {
            id: an_event(),
            paused,
        };
        let bytes = encode(&message).expect("encodes");
        assert_eq!(bytes[0], EVENT_PAUSE_VARIANT_TAG);
        assert_eq!(message.kind_name(), "EventPause");
        assert_eq!(decode(&bytes).expect("decodes"), message);
    }
}

#[test]
fn the_event_pause_is_relayed_reliable_and_not_latest_wins() {
    let message = Message::EventPause {
        id: an_event(),
        paused: true,
    };
    assert!(message.is_relayed());
    assert!(!message.is_latest_wins());
    assert!(!GameMessage::forwarded_to_other_guests(&message));
}

#[test]
fn an_event_pause_for_an_event_of_an_unknown_kind_is_refused() {
    let message = Message::EventPause {
        id: EventId {
            kind: 7,
            ..an_event()
        },
        paused: true,
    };
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "event" })
    );
}

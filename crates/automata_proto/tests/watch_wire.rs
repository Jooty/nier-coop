use automata_proto::{Message, PlayerState, decode, encode};

fn a_state() -> PlayerState {
    PlayerState::from_snapshot(&Default::default(), 1)
}

#[test]
fn a_state_made_from_a_snapshot_says_not_watching() {
    assert!(!a_state().watching);
}

#[test]
fn a_state_tells_whether_its_sender_watches() {
    for watching in [false, true] {
        let sent = PlayerState {
            watching,
            ..a_state()
        };
        assert_eq!(sent.validate(), Ok(()));
        let Ok(Message::State(decoded)) = decode(&encode(&Message::State(sent)).expect("encodes"))
        else {
            panic!("expected a state message");
        };
        assert_eq!(decoded.watching, watching);
    }
}

#[test]
fn the_watching_word_is_the_last_byte_of_a_state_message() {
    let not_watching = encode(&Message::State(a_state())).expect("encodes");
    let watching = encode(&Message::State(PlayerState {
        watching: true,
        ..a_state()
    }))
    .expect("encodes");
    assert_eq!(not_watching.len(), watching.len());
    let last = not_watching.len() - 1;
    assert_eq!(not_watching[..last], watching[..last]);
    assert_eq!((not_watching[last], watching[last]), (0, 1));
}

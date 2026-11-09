use rc_proto::{
    FightRequest, FightRequestKind, FightStreamEntry, FightWindowChange, FightWindowMark,
    MAX_MESSAGE_BYTES, Message, PROTOCOL_VERSION, StoryCallArg, WireError, decode, encode,
};

fn entry() -> FightStreamEntry {
    FightStreamEntry {
        phase: 23,
        epoch: 2,
        seq: 41,
        name: "EID_0200_a0037g_02_End".to_owned(),
        args: vec![
            StoryCallArg::Int(212),
            StoryCallArg::Number(7),
            StoryCallArg::Text(b"count_kill_enemy".to_vec()),
        ],
    }
}

fn mark(change: FightWindowChange) -> FightWindowMark {
    FightWindowMark {
        phase: 23,
        epoch: 2,
        number: 1,
        part: 1,
        change,
    }
}

#[test]
fn a_fight_stream_entry_round_trips_fits_a_message_and_is_relayed() {
    let message = Message::FightStream(entry());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    let bytes = encode(&message).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes), Ok(message));
    assert_eq!(PROTOCOL_VERSION, 19);
}

#[test]
fn a_fight_stream_entry_with_the_largest_arguments_fits_a_message() {
    let message = Message::FightStream(FightStreamEntry {
        seq: u64::MAX,
        epoch: u32::MAX,
        name: "x".repeat(63),
        args: vec![StoryCallArg::Text(vec![b'a'; 48]); 6],
        ..entry()
    });
    assert_eq!(message.validate(), Ok(()));
    assert!(encode(&message).unwrap().len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn a_fight_stream_entry_with_a_bad_name_or_arguments_is_refused() {
    for name in [String::new(), "two words".to_owned(), "x".repeat(64)] {
        let bad = FightStreamEntry { name, ..entry() };
        assert_eq!(
            bad.validate(),
            Err(WireError::OutOfRange { field: "function" })
        );
    }
    let too_many = FightStreamEntry {
        args: vec![StoryCallArg::Int(0); 7],
        ..entry()
    };
    assert_eq!(
        too_many.validate(),
        Err(WireError::OutOfRange { field: "args" })
    );
}

#[test]
fn a_fight_window_mark_round_trips_and_is_relayed_for_open_and_close() {
    for change in [
        FightWindowChange::Opened,
        FightWindowChange::Restarted,
        FightWindowChange::Closed { end_seq: 120 },
    ] {
        let message = Message::FightWindow(mark(change));
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_fight_window_mark_without_a_fight_number_is_refused() {
    let bad = FightWindowMark {
        number: 0,
        ..mark(FightWindowChange::Opened)
    };
    assert_eq!(
        bad.validate(),
        Err(WireError::OutOfRange {
            field: "fight window"
        })
    );
}

#[test]
fn a_fight_request_for_a_pop_or_a_restart_round_trips_fits_a_message_and_is_relayed() {
    for kind in [
        FightRequestKind::RunPop {
            name: "x".repeat(63),
            args: vec![StoryCallArg::Text(vec![b'a'; 48]); 6],
        },
        FightRequestKind::RestartPhase,
    ] {
        let message = Message::FightRequest(FightRequest {
            phase: 23,
            epoch: u32::MAX,
            kind,
        });
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        let bytes = encode(&message).unwrap();
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn a_fight_request_with_a_bad_name_is_refused() {
    let bad = FightRequest {
        phase: 23,
        epoch: 2,
        kind: FightRequestKind::RunPop {
            name: "two words".to_owned(),
            args: Vec::new(),
        },
    };
    assert_eq!(
        bad.validate(),
        Err(WireError::OutOfRange { field: "function" })
    );
}

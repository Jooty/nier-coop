use rc_proto::{
    DoorAction, DoorChange, DoorReport, DoorState, MAX_DOOR_LAYOUT_ID, MAX_DOORS_PER_REPORT,
    MAX_MESSAGE_BYTES, Message, decode, encode,
};

fn change() -> DoorChange {
    DoorChange {
        layout_id: 601,
        action: DoorAction::Open,
        flag: true,
        phase: 12,
    }
}

fn full_report() -> DoorReport {
    DoorReport {
        phase: 12,
        doors: (0..MAX_DOORS_PER_REPORT)
            .map(|index| DoorState {
                layout_id: MAX_DOOR_LAYOUT_ID - u16::try_from(index).unwrap_or(0),
                open: true,
                locked: true,
            })
            .collect(),
    }
}

#[test]
fn door_messages_survive_a_round_trip_and_are_relayed() {
    for message in [
        Message::DoorChange(change()),
        Message::DoorRequest(change()),
        Message::DoorReport(full_report()),
    ] {
        let bytes = encode(&message).expect("valid door message encodes");
        assert_eq!(decode(&bytes), Ok(message.clone()));
        assert!(message.is_relayed());
    }
}

#[test]
fn a_full_door_report_fits_one_message() {
    let bytes = encode(&Message::DoorReport(full_report())).expect("full report encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES, "{} bytes", bytes.len());
}

#[test]
fn nonsense_door_messages_are_refused() {
    let bad_id = DoorChange {
        layout_id: MAX_DOOR_LAYOUT_ID + 1,
        ..change()
    };
    let mut too_many = full_report();
    too_many.doors.push(too_many.doors[0]);
    assert!(encode(&Message::DoorChange(bad_id)).is_err());
    assert!(encode(&Message::DoorRequest(bad_id)).is_err());
    assert!(encode(&Message::DoorReport(too_many)).is_err());
}

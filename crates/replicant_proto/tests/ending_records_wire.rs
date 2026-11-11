use replicant_proto::{
    ENDING_ACCOMPLISHED_BYTES, EndingRecords, Message, WireError, decode, encode,
};

fn records() -> EndingRecords {
    EndingRecords {
        clear_bits: 0b1001,
        costumes: [1, 0, 0, 0, 0, 0, 1],
        accomplished: vec![0; ENDING_ACCOMPLISHED_BYTES],
    }
}

#[test]
fn ending_records_round_trip_and_are_relayed() {
    let message = Message::EndingRecords(records());
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn ending_records_with_a_wrong_accomplished_length_are_refused() {
    let mut wrong = records();
    wrong.accomplished.pop();
    assert_eq!(
        Message::EndingRecords(wrong).validate(),
        Err(WireError::OutOfRange {
            field: "accomplished"
        })
    );
}

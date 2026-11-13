use replicant_proto::{MAX_PICKUP_PART_GROUP, Message, PartPickup, WireError, decode, encode};

#[test]
fn a_part_pickup_round_trips_and_is_relayed() {
    let message = Message::PartPickup(PartPickup {
        phase: 7,
        actor_handler_rva: 0x24_6890,
        part_group: 12,
    });
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "PartPickup");
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_part_group_past_the_model_name_range_is_refused() {
    let message = Message::PartPickup(PartPickup {
        phase: 7,
        actor_handler_rva: 0x24_6890,
        part_group: MAX_PICKUP_PART_GROUP + 1,
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange {
            field: "part_group"
        })
    );
}

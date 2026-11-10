use replicant_proto::{BlockMoved, MAX_BLOCK_ID, Message, decode, encode};

fn moved(id: u16, position: [f32; 3]) -> Message {
    Message::BlockMoved(BlockMoved {
        phase: 4,
        id,
        position,
    })
}

#[test]
fn a_block_moved_round_trips_and_is_relayed() {
    let message = moved(2, [-24.0, 150.5, 18.0]);
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_block_moved_with_a_non_finite_position_is_refused() {
    assert!(moved(0, [f32::NAN, 0.0, 0.0]).validate().is_err());
    assert!(moved(0, [0.0, f32::INFINITY, 0.0]).validate().is_err());
}

#[test]
fn a_block_moved_with_an_id_past_the_limit_is_refused() {
    assert_eq!(moved(MAX_BLOCK_ID, [0.0; 3]).validate(), Ok(()));
    assert!(moved(MAX_BLOCK_ID + 1, [0.0; 3]).validate().is_err());
}

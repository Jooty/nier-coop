use replicant_proto::{BombBlast, Message, PROTOCOL_VERSION, WallBroken, decode, encode};

fn wall(layout_id: Option<u16>, position: [f32; 3]) -> Message {
    Message::WallBroken(WallBroken {
        phase: 12,
        layout_id,
        position,
    })
}

fn blast(position: [f32; 3]) -> Message {
    Message::BombBlast(BombBlast {
        phase: 12,
        position,
    })
}

#[test]
fn a_broken_wall_round_trips_with_or_without_a_layout_id_and_is_relayed() {
    for layout_id in [Some(4021), None] {
        let message = wall(layout_id, [-35.0, 0.5, 105.0]);
        assert!(message.is_relayed());
        assert_eq!(message.kind_name(), "WallBroken");
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_broken_wall_somewhere_impossible_is_refused() {
    assert!(wall(None, [f32::NAN, 0.0, 0.0]).validate().is_err());
    assert!(wall(Some(1), [0.0, 2_000_000.0, 0.0]).validate().is_err());
}

#[test]
fn a_bomb_blast_round_trips_and_is_relayed() {
    let message = blast([-9.9, -0.5, 37.0]);
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "BombBlast");
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_bomb_blast_somewhere_impossible_is_refused() {
    assert!(blast([0.0, f32::INFINITY, 0.0]).validate().is_err());
}

#[test]
fn the_wall_and_bomb_messages_are_protocol_50() {
    assert_eq!(PROTOCOL_VERSION, 50);
}

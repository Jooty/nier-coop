use replicant_proto::{
    ENEMY_GROUP_COUNT, Message, WireError, WorldObjectBroken, WorldObjectKey, WorldObjectMoved,
    decode, encode,
};

fn cube_key() -> WorldObjectKey {
    WorldObjectKey {
        kind_vtable_rva: 0x00B0_54B0,
        group: 11,
        start_cell: [32, 12, 62],
    }
}

#[test]
fn a_world_object_move_round_trips_and_is_relayed() {
    let message = Message::WorldObjectMoved(WorldObjectMoved {
        phase: 5,
        key: cube_key(),
        position: [32.25, 11.95, 63.5],
        held: true,
    });
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "WorldObjectMoved");
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_world_object_break_round_trips_and_is_relayed() {
    let message = Message::WorldObjectBroken(WorldObjectBroken {
        phase: 5,
        key: cube_key(),
    });
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "WorldObjectBroken");
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_group_past_the_actor_manager_is_refused() {
    let message = Message::WorldObjectBroken(WorldObjectBroken {
        phase: 5,
        key: WorldObjectKey {
            group: ENEMY_GROUP_COUNT,
            ..cube_key()
        },
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "group" })
    );
}

#[test]
fn a_move_to_a_point_that_is_not_a_number_is_refused() {
    let message = Message::WorldObjectMoved(WorldObjectMoved {
        phase: 5,
        key: cube_key(),
        position: [f32::NAN, 11.95, 63.5],
        held: false,
    });
    assert_eq!(
        message.validate(),
        Err(WireError::OutOfRange { field: "position" })
    );
}

use replicant_proto::{
    ENEMY_SPEC_BYTES, EnemyCreation, EnemyId, MAX_MESSAGE_BYTES, Message, WireError, decode, encode,
};

fn creation() -> EnemyCreation {
    EnemyCreation {
        phase: 7,
        id: EnemyId {
            group: 100,
            ordinal: 1,
        },
        actor_name: "em0100".to_owned(),
        spec_slot: 12,
        spawn_flag: 1,
        attach_flag: 1,
        scale: 1.25,
        spec: vec![7; ENEMY_SPEC_BYTES],
    }
}

#[test]
fn a_creation_survives_a_round_trip_and_fits_the_message_limit() {
    let message = Message::EnemyCreation(creation());
    let bytes = encode(&message).expect("encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes).expect("decodes"), message);
    assert!(message.is_relayed());
    assert!(!message.is_latest_wins());
}

#[test]
fn a_creation_with_a_bad_field_is_refused() {
    let bad = [
        EnemyCreation {
            actor_name: String::new(),
            ..creation()
        },
        EnemyCreation {
            actor_name: "has space".to_owned(),
            ..creation()
        },
        EnemyCreation {
            actor_name: "x".repeat(33),
            ..creation()
        },
        EnemyCreation {
            spec_slot: 256,
            ..creation()
        },
        EnemyCreation {
            spec: vec![0; ENEMY_SPEC_BYTES - 1],
            ..creation()
        },
        EnemyCreation {
            scale: f32::NAN,
            ..creation()
        },
        EnemyCreation {
            scale: 0.0,
            ..creation()
        },
        EnemyCreation {
            id: EnemyId {
                group: u16::MAX,
                ordinal: 0,
            },
            ..creation()
        },
    ];
    for creation in bad {
        let result = encode(&Message::EnemyCreation(creation.clone()));
        assert!(
            matches!(result, Err(WireError::OutOfRange { .. })),
            "{creation:?}"
        );
    }
}

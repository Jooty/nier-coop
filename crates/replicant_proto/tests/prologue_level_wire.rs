use replicant_proto::{MAX_PROLOGUE_LEVEL, Message, PrologueLevel, WireError, decode, encode};

#[test]
fn a_prologue_level_round_trips_and_is_relayed() {
    let message = Message::PrologueLevel(PrologueLevel {
        level: 7,
        experience: 1234,
    });
    assert!(message.is_relayed());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn a_level_outside_the_range_is_refused() {
    for level in [-1, MAX_PROLOGUE_LEVEL + 1] {
        assert_eq!(
            PrologueLevel {
                level,
                experience: 0
            }
            .validate(),
            Err(WireError::OutOfRange { field: "level" })
        );
    }
}

#[test]
fn negative_experience_is_refused() {
    assert_eq!(
        PrologueLevel {
            level: 1,
            experience: -1
        }
        .validate(),
        Err(WireError::OutOfRange {
            field: "experience"
        })
    );
}

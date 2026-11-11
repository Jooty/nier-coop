use replicant_proto::{MAX_VOICE_CHARACTER_BYTES, Message, VoiceCharacter, decode, encode};

fn kaine() -> VoiceCharacter {
    VoiceCharacter {
        state: "kaine".to_owned(),
    }
}

#[test]
fn a_voice_character_round_trips_and_is_relayed() {
    let message = Message::VoiceCharacter(kaine());
    assert!(message.is_relayed());
    assert_eq!(message.kind_name(), "VoiceCharacter");
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
}

#[test]
fn an_empty_long_or_control_character_name_is_refused() {
    let empty = VoiceCharacter {
        state: String::new(),
    };
    let long = VoiceCharacter {
        state: "k".repeat(MAX_VOICE_CHARACTER_BYTES + 1),
    };
    let control = VoiceCharacter {
        state: "kai\nne".to_owned(),
    };
    assert!(empty.validate().is_err());
    assert!(long.validate().is_err());
    assert!(control.validate().is_err());
}

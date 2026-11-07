use rc_proto::{
    BulletEffectPlay, EFFECT_ATTACH_BYTES, EFFECT_PARAM_BYTES, EffectPlay, EffectStop, MAX_EFFECT_BONE_BYTES,
    MAX_EFFECT_NAME_BYTES, MAX_MESSAGE_BYTES, MAX_POSITION_ABS, MAX_SOUND_NAME_BYTES, Message, SoundPlay, WireError,
    decode, encode,
};

fn effect() -> EffectPlay {
    EffectPlay {
        effect: "ef_player_sample".to_owned(),
        params: vec![3; EFFECT_PARAM_BYTES],
        attach: vec![4; EFFECT_ATTACH_BYTES],
        bone: "Hand_R".to_owned(),
        second_bone: String::new(),
        handle: 7,
    }
}

#[test]
fn an_effect_and_a_sound_survive_a_round_trip() {
    for message in [
        Message::Effect(effect()),
        Message::Sound(SoundPlay {
            sound: "se_player_sample".to_owned(),
        }),
    ] {
        let bytes = encode(&message).expect("valid message encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn the_longest_effect_still_fits_one_message() {
    let longest = EffectPlay {
        effect: "e".repeat(MAX_EFFECT_NAME_BYTES),
        params: vec![0xFF; EFFECT_PARAM_BYTES],
        attach: vec![0xFF; EFFECT_ATTACH_BYTES],
        bone: "b".repeat(MAX_EFFECT_BONE_BYTES),
        second_bone: "c".repeat(MAX_EFFECT_BONE_BYTES),
        handle: u32::MAX,
    };
    let bytes = encode(&Message::Effect(longest)).expect("longest effect encodes");
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
}

#[test]
fn effects_and_sounds_are_relayed_between_games() {
    assert!(Message::Effect(effect()).is_relayed());
    assert!(
        Message::Sound(SoundPlay {
            sound: "s".to_owned()
        })
        .is_relayed()
    );
}

#[test]
fn an_effect_without_a_name_is_rejected() {
    let mut nameless = effect();
    nameless.effect.clear();
    assert_eq!(
        nameless.validate(),
        Err(WireError::OutOfRange { field: "effect" })
    );
}

#[test]
fn an_effect_may_have_no_bone() {
    let mut bare = effect();
    bare.bone.clear();
    assert_eq!(bare.validate(), Ok(()));
}

#[test]
fn an_effect_with_the_wrong_parameter_size_is_rejected() {
    let mut short = effect();
    short.params.pop();
    assert_eq!(
        short.validate(),
        Err(WireError::OutOfRange {
            field: "effect params"
        })
    );
}

#[test]
fn an_effect_with_the_wrong_attach_size_is_rejected() {
    let mut short = effect();
    short.attach.pop();
    assert_eq!(
        short.validate(),
        Err(WireError::OutOfRange {
            field: "effect attach"
        })
    );
}

#[test]
fn an_overlong_or_control_character_name_is_rejected() {
    let mut long = effect();
    long.effect = "e".repeat(MAX_EFFECT_NAME_BYTES + 1);
    assert!(long.validate().is_err());
    let mut control = effect();
    control.bone = "Hand\n".to_owned();
    assert!(control.validate().is_err());
    let sound = SoundPlay {
        sound: "s".repeat(MAX_SOUND_NAME_BYTES + 1),
    };
    assert!(sound.validate().is_err());
}

#[test]
fn an_invalid_effect_is_not_encoded() {
    let mut bad = effect();
    bad.params.clear();
    assert!(encode(&Message::Effect(bad)).is_err());
}

#[test]
fn an_effect_stop_survives_a_round_trip_and_is_relayed() {
    let message = Message::EffectStop(EffectStop { handle: 0x27E13E });
    let bytes = encode(&message).expect("valid message encodes");
    assert_eq!(decode(&bytes), Ok(message.clone()));
    assert!(message.is_relayed());
}

fn bullet_effect() -> BulletEffectPlay {
    BulletEffectPlay {
        position: [10.0, -2.5, 300.0],
        effect: effect(),
    }
}

#[test]
fn book_and_bullet_messages_survive_a_round_trip() {
    for message in [
        Message::BookEffect(effect()),
        Message::BookSound(SoundPlay {
            sound: "se_player_sample".to_owned(),
        }),
        Message::BulletEffect(bullet_effect()),
    ] {
        let bytes = encode(&message).expect("valid message encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

#[test]
fn book_and_bullet_messages_are_relayed_between_games() {
    assert!(Message::BookEffect(effect()).is_relayed());
    assert!(Message::BulletEffect(bullet_effect()).is_relayed());
    assert!(
        Message::BookSound(SoundPlay {
            sound: "s".to_owned()
        })
        .is_relayed()
    );
}

#[test]
fn a_bullet_effect_at_a_non_finite_or_far_position_is_rejected() {
    for bad in [f32::NAN, f32::INFINITY, MAX_POSITION_ABS + 1.0] {
        let mut bullet = bullet_effect();
        bullet.position[1] = bad;
        assert_eq!(
            Message::BulletEffect(bullet).validate(),
            Err(WireError::OutOfRange {
                field: "bullet effect position"
            })
        );
    }
}

#[test]
fn a_book_effect_without_a_name_is_rejected() {
    let mut nameless = effect();
    nameless.effect.clear();
    assert_eq!(
        Message::BookEffect(nameless).validate(),
        Err(WireError::OutOfRange { field: "effect" })
    );
}

use rc_proto::{
    EffectLook, MAX_POSITION_ABS, Message, WireError, WorldEffectMove, WorldEffectPlay, decode,
    encode,
};

fn played() -> WorldEffectPlay {
    WorldEffectPlay {
        effect: "efP_blackExecution_mc01".to_owned(),
        transform: [1.0, 0.0, 0.0, 4.0, 0.0, 1.0, 0.0, 5.0, 0.0, 0.0, 1.0, 6.0],
        arguments: [1.0_f32.to_bits(), 1, 0, 0, 0],
        handle: 0x44021,
        look: None,
    }
}

#[test]
fn a_world_effect_and_its_move_round_trip_and_are_relayed() {
    let moved = WorldEffectMove {
        handle: 0x44021,
        transform: played().transform,
    };
    for message in [
        Message::WorldEffect(played()),
        Message::WorldEffectMove(moved),
    ] {
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_world_effect_far_away_unnamed_or_not_finite_is_refused() {
    let mut far = played();
    far.transform[7] = MAX_POSITION_ABS + 1.0;
    let mut broken = played();
    broken.transform[0] = f32::NAN;
    let field = "world effect transform";
    assert_eq!(far.validate(), Err(WireError::OutOfRange { field }));
    assert_eq!(broken.validate(), Err(WireError::OutOfRange { field }));
    let unnamed = WorldEffectPlay {
        effect: String::new(),
        ..played()
    };
    assert!(unnamed.validate().is_err());
}

fn looked() -> EffectLook {
    EffectLook {
        style: [0.9, 0.2, 0.1, 1.0, 2.0, 2.0, 2.0, 2.0],
        extra: 1.5,
        flag: 1,
    }
}

#[test]
fn a_world_effect_carries_its_look_and_refuses_one_that_is_not_finite() {
    let with_look = WorldEffectPlay {
        look: Some(looked()),
        ..played()
    };
    let message = Message::WorldEffect(with_look.clone());
    assert_eq!(message.validate(), Ok(()));
    assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    let mut broken = looked();
    broken.style[3] = f32::NAN;
    let refused = WorldEffectPlay {
        look: Some(broken),
        ..with_look
    };
    assert_eq!(
        refused.validate(),
        Err(WireError::OutOfRange {
            field: "effect look"
        })
    );
}

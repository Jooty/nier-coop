use rc_proto::{
    MAX_SCRIPT_ROLL_ORDINAL, Message, ScriptRoll, ScriptRollValue, WireError, decode, encode,
};

fn roll(value: ScriptRollValue) -> ScriptRoll {
    ScriptRoll {
        phase: 7,
        call: 0xDEAD_BEEF_0123_4567,
        ordinal: 3,
        value,
    }
}

#[test]
fn a_script_roll_round_trips_and_is_relayed() {
    for value in [ScriptRollValue::Int(-4), ScriptRollValue::Float(0.25)] {
        let message = Message::ScriptRoll(roll(value));
        assert!(message.is_relayed());
        assert_eq!(message.validate(), Ok(()));
        assert_eq!(decode(&encode(&message).unwrap()), Ok(message));
    }
}

#[test]
fn a_script_roll_with_an_absurd_ordinal_or_a_non_finite_float_is_refused() {
    let mut far = roll(ScriptRollValue::Int(1));
    far.ordinal = MAX_SCRIPT_ROLL_ORDINAL + 1;
    assert_eq!(
        Message::ScriptRoll(far).validate(),
        Err(WireError::OutOfRange { field: "ordinal" })
    );
    for bad in [f32::NAN, f32::INFINITY] {
        assert_eq!(
            Message::ScriptRoll(roll(ScriptRollValue::Float(bad))).validate(),
            Err(WireError::OutOfRange { field: "value" })
        );
    }
}

use rc_proto::ScriptRollValue;

use crate::story_relay::{ARG_TAG_INT, ARG_TAG_NUMBER};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollNumber(pub f64);

pub fn number_from_slot(tag: i32, payload: [u8; 8]) -> Option<RollNumber> {
    match tag {
        ARG_TAG_INT => {
            let [a, b, c, d, ..] = payload;
            Some(RollNumber(f64::from(i32::from_le_bytes([a, b, c, d]))))
        }
        ARG_TAG_NUMBER => Some(RollNumber(f64::from_le_bytes(payload))),
        _ => None,
    }
}

pub fn value_fits(value: ScriptRollValue, low: RollNumber, high: RollNumber) -> bool {
    let (low, high) = (low.0.min(high.0), low.0.max(high.0));
    let number = match value {
        ScriptRollValue::Int(int) => f64::from(int),
        ScriptRollValue::Float(float) => f64::from(float),
    };
    number >= low && number <= high
}

pub fn value_fits_arguments(
    value: ScriptRollValue,
    argument_count: i32,
    low: Option<RollNumber>,
    high: Option<RollNumber>,
) -> bool {
    if argument_count >= 2 {
        return match (low, high) {
            (Some(low), Some(high)) => value_fits(value, low, high),
            _ => false,
        };
    }
    match value {
        ScriptRollValue::Float(float) => (0.0..1.0).contains(&float),
        ScriptRollValue::Int(_) => false,
    }
}

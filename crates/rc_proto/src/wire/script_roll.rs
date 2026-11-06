use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_SCRIPT_ROLL_ORDINAL: u16 = 4_096;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ScriptRollValue {
    Int(i32),
    Float(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScriptRoll {
    pub phase: u16,
    pub call: u64,
    pub ordinal: u16,
    pub value: ScriptRollValue,
}

impl ScriptRoll {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.ordinal > MAX_SCRIPT_ROLL_ORDINAL {
            return Err(WireError::OutOfRange { field: "ordinal" });
        }
        if let ScriptRollValue::Float(value) = self.value
            && !value.is_finite()
        {
            return Err(WireError::OutOfRange { field: "value" });
        }
        Ok(())
    }
}

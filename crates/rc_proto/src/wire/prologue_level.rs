use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_PROLOGUE_LEVEL: i32 = 99;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrologueLevel {
    pub level: i32,
    pub experience: i32,
}

impl PrologueLevel {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(0..=MAX_PROLOGUE_LEVEL).contains(&self.level) {
            return Err(WireError::OutOfRange { field: "level" });
        }
        if self.experience < 0 {
            return Err(WireError::OutOfRange {
                field: "experience",
            });
        }
        Ok(())
    }
}

use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const ENDING_COSTUME_BYTES: usize = 7;
pub const ENDING_ACCOMPLISHED_BYTES: usize = 0x2F;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndingRecords {
    pub clear_bits: u32,
    pub costumes: [u8; ENDING_COSTUME_BYTES],
    pub accomplished: Vec<u8>,
}

impl EndingRecords {
    pub fn validate(&self) -> Result<(), WireError> {
        if self.accomplished.len() != ENDING_ACCOMPLISHED_BYTES {
            return Err(WireError::OutOfRange {
                field: "accomplished",
            });
        }
        Ok(())
    }
}

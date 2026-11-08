use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const MAX_VIEW_CHAIN_BYTES: usize = 32;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewState {
    pub chain: String,
    pub inside: bool,
}

impl ViewState {
    pub fn validate(&self) -> Result<(), WireError> {
        let bytes = self.chain.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_VIEW_CHAIN_BYTES
            || !bytes.iter().all(u8::is_ascii_graphic)
        {
            return Err(WireError::OutOfRange {
                field: "view chain",
            });
        }
        Ok(())
    }
}

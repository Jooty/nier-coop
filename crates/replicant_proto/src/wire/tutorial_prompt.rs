use serde::{Deserialize, Serialize};

use crate::wire::WireError;

pub const FIRST_TUTORIAL_ID: u16 = 2000;
pub const LAST_TUTORIAL_ID: u16 = 2127;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TutorialPrompt {
    pub id: u16,
}

impl TutorialPrompt {
    pub fn validate(&self) -> Result<(), WireError> {
        if !(FIRST_TUTORIAL_ID..=LAST_TUTORIAL_ID).contains(&self.id) {
            return Err(WireError::OutOfRange { field: "id" });
        }
        Ok(())
    }
}

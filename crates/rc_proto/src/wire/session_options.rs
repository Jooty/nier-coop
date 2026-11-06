use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOptions {
    pub show_names: bool,
    pub show_party_frame: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            show_names: true,
            show_party_frame: true,
        }
    }
}

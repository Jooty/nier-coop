use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOptions {
    pub show_names: bool,
    pub show_party_frame: bool,
    pub show_distance: bool,
    pub show_ping: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self {
            show_names: true,
            show_party_frame: true,
            show_distance: false,
            show_ping: false,
        }
    }
}

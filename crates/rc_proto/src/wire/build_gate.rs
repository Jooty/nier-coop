use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildGate {
    pub time_date_stamp: u32,
    pub size_of_image: u32,
}

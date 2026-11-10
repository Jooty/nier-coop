use serde::{Deserialize, Serialize};

use crate::wire::PartnerBody;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobbyBody {
    pub body: PartnerBody,
}

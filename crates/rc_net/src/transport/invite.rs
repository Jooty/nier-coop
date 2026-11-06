use crate::transport::{MAX_INVITE_BYTES, NetError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invite(String);

impl Invite {
    pub fn new(text: String) -> Result<Self, NetError> {
        if text.is_empty() || text.len() > MAX_INVITE_BYTES {
            return Err(NetError::InvalidInvite);
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

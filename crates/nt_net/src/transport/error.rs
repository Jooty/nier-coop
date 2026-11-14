use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum NetError {
    #[error("invite is empty, too long or not one this transport issued")]
    InvalidInvite,
    #[error("not connected to that peer")]
    NotConnected,
    #[error("payload is {len} bytes, the limit is {limit}")]
    PayloadTooLarge { len: usize, limit: usize },
    #[error("transport failure: {0}")]
    Transport(String),
}

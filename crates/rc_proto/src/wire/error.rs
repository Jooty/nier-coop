use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WireError {
    #[error("empty message")]
    Empty,
    #[error("message is {len} bytes, the limit is {limit}")]
    TooLarge { len: usize, limit: usize },
    #[error("message ends early")]
    Truncated,
    #[error("message is not a valid encoding")]
    Malformed,
    #[error("{extra} bytes after the end of the message")]
    TrailingBytes { extra: usize },
    #[error("{field} is out of range")]
    OutOfRange { field: &'static str },
}

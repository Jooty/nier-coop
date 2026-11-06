use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NetError {
    #[error("{0}")]
    Failed(String),
    #[error("networking is not available in this build")]
    Unavailable,
}

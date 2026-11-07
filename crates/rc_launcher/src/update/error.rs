use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("could not reach the update server: {0}")]
    Network(String),
    #[error("update server refused the request ({0})")]
    ServerRefused(u16),
    #[error("the update manifest is not valid: {0}")]
    BadManifest(String),
    #[error("the update manifest signature does not match")]
    BadSignature,
    #[error("the update has no {0} for this system")]
    MissingFile(String),
    #[error("{0} does not match its manifest entry")]
    FileMismatch(String),
    #[error("the server sent an old copy of {0} that does not match its manifest entry")]
    StaleCopy(String),
    #[error("could not write the update: {0}")]
    Io(String),
}

impl From<std::io::Error> for UpdateError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

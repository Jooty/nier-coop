use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReleaseError {
    #[error("could not read or write {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the key file {0} is not a 64 character hex seed")]
    BadKeyFile(PathBuf),
    #[error("{0} already exists, refusing to overwrite a key")]
    KeyFileExists(PathBuf),
    #[error("no system randomness available: {0}")]
    NoRandomness(String),
    #[error("bad file spec {0:?}, expected <path>:<launcher|hook|hook@game>:<platform>")]
    BadFileSpec(String),
    #[error("could not write the manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Usage(String),
}

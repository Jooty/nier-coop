use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::Path;

use ed25519_dalek::SigningKey;

use crate::error::ReleaseError;
use crate::key::{KEY_SEED_BYTES, encode_hex};

pub fn generate_key_file(path: &Path) -> Result<String, ReleaseError> {
    let mut seed = [0_u8; KEY_SEED_BYTES];
    getrandom::fill(&mut seed).map_err(|error| ReleaseError::NoRandomness(error.to_string()))?;
    let key = SigningKey::from_bytes(&seed);
    let io_error = |source| ReleaseError::Io {
        path: path.to_owned(),
        source,
    };
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let mut file = open_new_private_file(path).map_err(|error| match error.kind() {
        ErrorKind::AlreadyExists => ReleaseError::KeyFileExists(path.to_owned()),
        _ => io_error(error),
    })?;
    writeln!(file, "{}", encode_hex(&seed)).map_err(io_error)?;
    Ok(encode_hex(key.verifying_key().as_bytes()))
}

#[cfg(unix)]
fn open_new_private_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    use crate::key::KEY_FILE_MODE;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(KEY_FILE_MODE)
        .open(path)
}

#[cfg(not(unix))]
fn open_new_private_file(path: &Path) -> std::io::Result<fs::File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

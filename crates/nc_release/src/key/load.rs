use std::fs;
use std::path::Path;

use ed25519_dalek::SigningKey;

use crate::error::ReleaseError;
use crate::key::{KEY_SEED_BYTES, decode_hex};

pub fn load_signing_key(path: &Path) -> Result<SigningKey, ReleaseError> {
    let text = fs::read_to_string(path).map_err(|source| ReleaseError::Io {
        path: path.to_owned(),
        source,
    })?;
    let seed: [u8; KEY_SEED_BYTES] = decode_hex(text.trim())
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| ReleaseError::BadKeyFile(path.to_owned()))?;
    Ok(SigningKey::from_bytes(&seed))
}

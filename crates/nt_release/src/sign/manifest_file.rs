use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signer, SigningKey};

use crate::error::ReleaseError;
use crate::key::load_signing_key;
use crate::sign::SIGNATURE_SUFFIX;

pub fn sign_bytes(key: &SigningKey, bytes: &[u8]) -> Vec<u8> {
    key.sign(bytes).to_bytes().to_vec()
}

pub fn sign_manifest_file(key_file: &Path, manifest: &Path) -> Result<PathBuf, ReleaseError> {
    let key = load_signing_key(key_file)?;
    let bytes = fs::read(manifest).map_err(|source| ReleaseError::Io {
        path: manifest.to_owned(),
        source,
    })?;
    let mut signature_name = OsString::from(manifest);
    signature_name.push(SIGNATURE_SUFFIX);
    let signature_path = PathBuf::from(signature_name);
    fs::write(&signature_path, sign_bytes(&key, &bytes)).map_err(|source| ReleaseError::Io {
        path: signature_path.clone(),
        source,
    })?;
    Ok(signature_path)
}

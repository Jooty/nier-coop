use ed25519_dalek::{Signature, VerifyingKey};

use crate::update::UpdateError;

pub fn verify_manifest_signature(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    public_key_hex: &str,
) -> Result<(), UpdateError> {
    let key_bytes: [u8; 32] = decode_hex(public_key_hex)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(UpdateError::BadSignature)?;
    let key = VerifyingKey::from_bytes(&key_bytes).map_err(|_| UpdateError::BadSignature)?;
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| UpdateError::BadSignature)?;
    key.verify_strict(manifest_bytes, &Signature::from_bytes(&signature_array))
        .map_err(|_| UpdateError::BadSignature)
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|start| u8::from_str_radix(&text[start..start + 2], 16).ok())
        .collect()
}

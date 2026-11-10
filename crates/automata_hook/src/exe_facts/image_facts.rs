use automata_game::exe::ExeFacts;
use nc_hook::build_gate::{PE_HEADER_READ_LEN, PeError, PeIdentity};
use sha2::{Digest, Sha256};

pub fn facts_from_image(image: &[u8]) -> Result<ExeFacts, PeError> {
    let header = &image[..image.len().min(PE_HEADER_READ_LEN)];
    let identity = PeIdentity::parse(header)?;
    Ok(ExeFacts {
        file_size: image.len() as u64,
        time_date_stamp: identity.time_date_stamp,
        sha256_hex: sha256_hex(image),
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

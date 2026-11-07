use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::update::fetch::open_download;
use crate::update::{ManifestFile, UpdateError};

const COPY_CHUNK_BYTES: usize = 64 * 1024;
const RETRY_QUERY_KEY: &str = "retry";

pub fn verify_file(path: &Path, expected: &ManifestFile) -> Result<(), UpdateError> {
    let mismatch = || UpdateError::FileMismatch(expected.name.clone());
    if std::fs::metadata(path)?.len() != expected.size {
        return Err(mismatch());
    }
    let actual = crate::install::sha256_file(path)?;
    if actual.eq_ignore_ascii_case(&expected.sha256) {
        Ok(())
    } else {
        Err(mismatch())
    }
}

pub fn with_query(url: &str, key: &str, value: &str) -> String {
    let separator = if url.contains('?') { '&' } else { '?' };
    format!("{url}{separator}{key}={value}")
}

pub fn download_verified(
    url: &str,
    expected: &ManifestFile,
    destination: &Path,
) -> Result<(), UpdateError> {
    match download_once(url, expected, destination) {
        Err(UpdateError::FileMismatch(_)) => {
            let unique = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos());
            let retry_url = with_query(url, RETRY_QUERY_KEY, &unique.to_string());
            download_once(&retry_url, expected, destination).map_err(|error| match error {
                UpdateError::FileMismatch(name) => UpdateError::StaleCopy(name),
                other => other,
            })
        }
        other => other,
    }
}

fn download_once(
    url: &str,
    expected: &ManifestFile,
    destination: &Path,
) -> Result<(), UpdateError> {
    let mut reader = open_download(url)?.take(expected.size.saturating_add(1));
    let mut file = std::fs::File::create(destination)?;
    let mut hasher = Sha256::new();
    let mut chunk = vec![0u8; COPY_CHUNK_BYTES];
    let mut written: u64 = 0;
    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&chunk[..read]);
        file.write_all(&chunk[..read])?;
        written += read as u64;
    }
    drop(file);
    if written != expected.size {
        let _ = std::fs::remove_file(destination);
        return Err(UpdateError::FileMismatch(expected.name.clone()));
    }
    verify_file(destination, expected).inspect_err(|_| {
        let _ = std::fs::remove_file(destination);
    })
}

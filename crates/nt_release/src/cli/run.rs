use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::{Command, parse_command};
use crate::error::ReleaseError;
use crate::key::generate_key_file;
use crate::manifest::{ManifestInputs, build_manifest, write_manifest};
use crate::sign::sign_manifest_file;

pub fn run(arguments: &[String]) -> Result<String, ReleaseError> {
    match parse_command(arguments)? {
        Command::Keygen { out } => generate_key_file(&out),
        Command::Manifest {
            channel,
            commit,
            commit_time,
            files,
            out,
        } => {
            let manifest = build_manifest(&ManifestInputs {
                channel,
                commit,
                commit_time,
                built_at: unix_seconds_now(),
                files,
            })?;
            write_manifest(&manifest, &out)?;
            Ok(format!(
                "wrote {} with {} files",
                out.display(),
                manifest.files.len()
            ))
        }
        Command::Sign { key_file, manifest } => {
            let signature = sign_manifest_file(&key_file, &manifest)?;
            Ok(format!("wrote {}", signature.display()))
        }
    }
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

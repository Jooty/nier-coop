use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::error::ReleaseError;
use crate::manifest::{FileSpec, ManifestFile, describe_file};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Manifest {
    pub channel: String,
    pub commit: String,
    pub commit_time: u64,
    pub built_at: u64,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone)]
pub struct ManifestInputs {
    pub channel: String,
    pub commit: String,
    pub commit_time: u64,
    pub built_at: u64,
    pub files: Vec<FileSpec>,
}

pub fn build_manifest(inputs: &ManifestInputs) -> Result<Manifest, ReleaseError> {
    let files = inputs
        .files
        .iter()
        .map(describe_file)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Manifest {
        channel: inputs.channel.clone(),
        commit: inputs.commit.clone(),
        commit_time: inputs.commit_time,
        built_at: inputs.built_at,
        files,
    })
}

pub fn write_manifest(manifest: &Manifest, path: &Path) -> Result<(), ReleaseError> {
    let bytes = serde_json::to_vec(manifest)?;
    fs::write(path, bytes).map_err(|source| ReleaseError::Io {
        path: path.to_owned(),
        source,
    })
}

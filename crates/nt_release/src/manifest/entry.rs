use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::ReleaseError;
use crate::key::encode_hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Launcher,
    Hook,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSpec {
    pub path: PathBuf,
    pub kind: FileKind,
    pub game: Option<String>,
    pub platform: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestFile {
    pub name: String,
    pub kind: FileKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    pub platform: String,
    pub size: u64,
    pub sha256: String,
}

pub fn parse_file_spec(text: &str) -> Result<FileSpec, ReleaseError> {
    let bad = || ReleaseError::BadFileSpec(text.to_owned());
    let mut parts = text.rsplitn(3, ':');
    let platform = parts.next().filter(|platform| !platform.is_empty());
    let (kind, game) = match parts.next() {
        Some("launcher") => (FileKind::Launcher, None),
        Some("hook") => (FileKind::Hook, None),
        Some(text) => match text.strip_prefix("hook@") {
            Some(game) if !game.is_empty() => (FileKind::Hook, Some(game.to_owned())),
            _ => return Err(bad()),
        },
        None => return Err(bad()),
    };
    let path = parts.next().filter(|path| !path.is_empty());
    match (path, platform) {
        (Some(path), Some(platform)) => Ok(FileSpec {
            path: PathBuf::from(path),
            kind,
            game,
            platform: platform.to_owned(),
        }),
        _ => Err(bad()),
    }
}

pub fn describe_file(spec: &FileSpec) -> Result<ManifestFile, ReleaseError> {
    let bytes = fs::read(&spec.path).map_err(|source| ReleaseError::Io {
        path: spec.path.clone(),
        source,
    })?;
    let name = file_name_of(&spec.path)
        .ok_or_else(|| ReleaseError::BadFileSpec(spec.path.display().to_string()))?;
    Ok(ManifestFile {
        name,
        kind: spec.kind,
        game: spec.game.clone(),
        platform: spec.platform.clone(),
        size: bytes.len() as u64,
        sha256: encode_hex(&Sha256::digest(&bytes)),
    })
}

fn file_name_of(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_owned)
}

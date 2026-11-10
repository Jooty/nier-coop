use serde::Deserialize;

use crate::update::UpdateError;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Manifest {
    pub channel: String,
    pub commit: String,
    #[serde(default)]
    pub commit_time: u64,
    #[serde(default)]
    pub built_at: u64,
    pub files: Vec<ManifestFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ManifestFile {
    pub name: String,
    pub kind: FileKind,
    pub platform: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Launcher,
    Hook,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    pub launcher: ManifestFile,
    pub hook: ManifestFile,
}

pub fn parse_manifest(bytes: &[u8]) -> Result<Manifest, UpdateError> {
    let manifest: Manifest = serde_json::from_slice(bytes)
        .map_err(|error| UpdateError::BadManifest(error.to_string()))?;
    if manifest.commit.is_empty() {
        return Err(UpdateError::BadManifest("no commit".to_owned()));
    }
    if let Some(file) = manifest
        .files
        .iter()
        .find(|file| !is_plain_file_name(&file.name))
    {
        return Err(UpdateError::BadManifest(format!(
            "unsafe file name {:?}",
            file.name
        )));
    }
    Ok(manifest)
}

pub fn pick_files(manifest: &Manifest, platform: &str) -> Result<UpdatePlan, UpdateError> {
    let find = |kind: FileKind, wanted_platform: &str| {
        manifest
            .files
            .iter()
            .find(|file| file.kind == kind && file.platform == wanted_platform)
            .cloned()
    };
    let launcher = find(FileKind::Launcher, platform)
        .ok_or_else(|| UpdateError::MissingFile(format!("launcher for {platform}")))?;
    let hook = find(FileKind::Hook, "windows")
        .ok_or_else(|| UpdateError::MissingFile("hook dll".to_owned()))?;
    Ok(UpdatePlan { launcher, hook })
}

fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && name != ".." && name != "." && !name.contains(['/', '\\', ':', '\0'])
}

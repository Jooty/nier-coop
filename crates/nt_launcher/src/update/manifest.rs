use nt_launcher_core::default_profile;
use serde::Deserialize;

use crate::update::UpdateError;

const HOOK_PLATFORM: &str = "windows";

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
    #[serde(default)]
    pub game: Option<String>,
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
pub struct GameHook {
    pub game_id: String,
    pub file: ManifestFile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlan {
    pub launcher: ManifestFile,
    pub hooks: Vec<GameHook>,
}

impl UpdatePlan {
    pub fn hook_for(&self, game_id: &str) -> Option<&ManifestFile> {
        self.hooks
            .iter()
            .find(|hook| hook.game_id == game_id)
            .map(|hook| &hook.file)
    }
}

impl ManifestFile {
    pub fn game_id(&self) -> &str {
        self.game.as_deref().unwrap_or(default_profile().game_id)
    }
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
    let launcher = manifest
        .files
        .iter()
        .find(|file| file.kind == FileKind::Launcher && file.platform == platform)
        .cloned()
        .ok_or_else(|| UpdateError::MissingFile(format!("launcher for {platform}")))?;
    let mut hooks: Vec<GameHook> = Vec::new();
    for file in manifest
        .files
        .iter()
        .filter(|file| file.kind == FileKind::Hook && file.platform == HOOK_PLATFORM)
    {
        if !hooks.iter().any(|hook| hook.game_id == file.game_id()) {
            hooks.push(GameHook {
                game_id: file.game_id().to_owned(),
                file: file.clone(),
            });
        }
    }
    if hooks.is_empty() {
        return Err(UpdateError::MissingFile("hook dll".to_owned()));
    }
    Ok(UpdatePlan { launcher, hooks })
}

fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && name != ".." && name != "." && !name.contains(['/', '\\', ':', '\0'])
}

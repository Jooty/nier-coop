use std::path::{Path, PathBuf};
use std::process::Command;

use nc_launcher_core::GameProfile;

use crate::steam::hide_console_window;

const REPO: &str = "Jooty/replicant-coop";
const WORKFLOW_FILE: &str = "ci.yml";
const ARTIFACT_NAME_SUFFIX: &str = "-hook-dll";
const BRANCH: &str = "main";
const RUN_LIST_LIMIT: &str = "30";

pub fn dll_next_to_executable(profile: &GameProfile) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let candidate = exe.parent()?.join(profile.hook_dll_name);
    candidate.is_file().then_some(candidate)
}

pub fn newer_dll(first: Option<PathBuf>, second: Option<PathBuf>) -> Option<PathBuf> {
    let modified = |path: &PathBuf| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
    };
    match (first, second) {
        (Some(a), Some(b)) => Some(if modified(&b) > modified(&a) { b } else { a }),
        (a, b) => a.or(b),
    }
}

pub fn newest_cached_ci_dll(profile: &GameProfile, cache_dir: &Path) -> Option<PathBuf> {
    let mut runs: Vec<(u64, PathBuf)> = std::fs::read_dir(cache_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let run_id = entry.file_name().to_str()?.parse().ok()?;
            Some((run_id, entry.path().join(profile.hook_dll_name)))
        })
        .filter(|(_, dll)| dll.is_file())
        .collect();
    runs.sort();
    runs.pop().map(|(_, dll)| dll)
}

pub fn ci_artifact_name(profile: &GameProfile) -> String {
    format!("{}{ARTIFACT_NAME_SUFFIX}", profile.game_id)
}

pub fn fetch_dll_from_ci(profile: &GameProfile, cache_dir: &Path) -> Result<PathBuf, String> {
    let artifact_name = ci_artifact_name(profile);
    let run_id = newest_run_with_artifact(&artifact_name)?;
    let target_dir = cache_dir.join(&run_id);
    let dll = target_dir.join(profile.hook_dll_name);
    if dll.is_file() {
        return Ok(dll);
    }
    std::fs::create_dir_all(&target_dir).map_err(|error| error.to_string())?;
    run_gh(&[
        "run",
        "download",
        &run_id,
        "-R",
        REPO,
        "--name",
        &artifact_name,
        "--dir",
        &target_dir.to_string_lossy(),
    ])?;
    dll.is_file()
        .then_some(dll)
        .ok_or_else(|| format!("the CI artifact held no {}", profile.hook_dll_name))
}

fn newest_run_with_artifact(artifact_name: &str) -> Result<String, String> {
    let listing = run_gh(&[
        "run",
        "list",
        "-R",
        REPO,
        "--workflow",
        WORKFLOW_FILE,
        "--status",
        "success",
        "--branch",
        BRANCH,
        "--limit",
        RUN_LIST_LIMIT,
        "--json",
        "databaseId",
        "--jq",
        ".[].databaseId",
    ])?;
    for run_id in listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let filter =
            format!(".artifacts[] | select(.name==\"{artifact_name}\" and .expired==false) | .id");
        let artifacts = run_gh(&[
            "api",
            &format!("repos/{REPO}/actions/runs/{run_id}/artifacts"),
            "--jq",
            &filter,
        ])?;
        if !artifacts.trim().is_empty() {
            return Ok(run_id.to_owned());
        }
    }
    Err(format!(
        "no successful {WORKFLOW_FILE} run on {BRANCH} has the {artifact_name} artifact"
    ))
}

fn run_gh(args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("gh");
    command.args(args);
    hide_console_window(&mut command);
    let output = command
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

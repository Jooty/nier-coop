use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

use crate::proton::{
    DLL_OVERRIDE_VALUE, USER_REG_BACKUP_SUFFIX, find_proton_files_dir, prefix_is_busy,
};
use crate::steam::GameInstall;
use nt_launcher_core::GameProfile;

#[derive(Debug, Error)]
pub enum OverrideError {
    #[error("the Proton prefix has running wine processes; close the game and Proton apps first")]
    PrefixBusy,
    #[error("could not find the Proton build for this game; start the game once from Steam")]
    NoProton,
    #[error("wine failed: {0}")]
    Wine(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

struct ProtonTools {
    wine: PathBuf,
    wineserver: PathBuf,
    prefix: PathBuf,
}

impl ProtonTools {
    fn run_wine(&self, args: &[&str]) -> Result<String, OverrideError> {
        let output = Command::new(&self.wine)
            .args(args)
            .env("WINEPREFIX", &self.prefix)
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(OverrideError::Wine(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }

    fn backup_user_reg(&self) -> std::io::Result<()> {
        let user_reg = self.prefix.join("user.reg");
        let backup = self
            .prefix
            .join(format!("user.reg.{USER_REG_BACKUP_SUFFIX}"));
        std::fs::copy(user_reg, backup).map(|_| ())
    }

    fn wait_for_wineserver(&self) {
        let _ = Command::new(&self.wineserver)
            .arg("-w")
            .env("WINEPREFIX", &self.prefix)
            .status();
    }
}

pub fn override_key(exe_name: &str) -> String {
    format!("HKCU\\Software\\Wine\\AppDefaults\\{exe_name}\\DllOverrides")
}

pub fn override_keys(profile: &GameProfile) -> Vec<String> {
    profile
        .exe_names
        .iter()
        .map(|exe_name| override_key(exe_name))
        .collect()
}

/// The key's parent, which `reg add` makes together with the key.
pub fn exe_defaults_key(exe_name: &str) -> String {
    format!("HKCU\\Software\\Wine\\AppDefaults\\{exe_name}")
}

/// True when `reg query` printed the key's own line and nothing else: no value and no subkey.
pub fn key_is_empty(reg_query_output: &str) -> bool {
    reg_query_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
        == 1
}

pub fn override_is_set(reg_query_output: &str) -> bool {
    reg_query_output.contains(DLL_OVERRIDE_VALUE)
}

pub fn add_override(game: &GameInstall) -> Result<bool, OverrideError> {
    let Some(tools) = tools_for(game)? else {
        return Ok(false);
    };
    let stem = game.profile.loader_dll_stem();
    let missing: Vec<String> = override_keys(game.profile)
        .into_iter()
        .filter(|key| {
            let current = tools
                .run_wine(&["reg", "query", key, "/v", stem])
                .unwrap_or_default();
            !override_is_set(&current)
        })
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }
    tools.backup_user_reg()?;
    for key in &missing {
        tools.run_wine(&[
            "reg",
            "add",
            key,
            "/v",
            stem,
            "/d",
            DLL_OVERRIDE_VALUE,
            "/f",
        ])?;
    }
    tools.wait_for_wineserver();
    Ok(true)
}

pub fn remove_override(game: &GameInstall) -> Result<bool, OverrideError> {
    let Some(tools) = tools_for(game)? else {
        return Ok(false);
    };
    tools.backup_user_reg()?;
    for key in override_keys(game.profile) {
        let _ = tools.run_wine(&[
            "reg",
            "delete",
            &key,
            "/v",
            game.profile.loader_dll_stem(),
            "/f",
        ]);
    }
    tools.wait_for_wineserver();
    Ok(true)
}

/// Deletes the override keys of this game, and their parents, where they hold nothing; returns
/// the keys deleted. Keys with any value or subkey are left as they are.
pub fn remove_empty_override_keys(game: &GameInstall) -> Result<Vec<String>, OverrideError> {
    let Some(tools) = tools_for(game)? else {
        return Ok(Vec::new());
    };
    let mut deleted = Vec::new();
    for exe_name in game.profile.exe_names {
        for key in [override_key(exe_name), exe_defaults_key(exe_name)] {
            let is_empty = tools
                .run_wine(&["reg", "query", &key])
                .is_ok_and(|output| key_is_empty(&output));
            if !is_empty {
                break;
            }
            tools.run_wine(&["reg", "delete", &key, "/f"])?;
            deleted.push(key);
        }
    }
    tools.wait_for_wineserver();
    Ok(deleted)
}

fn tools_for(game: &GameInstall) -> Result<Option<ProtonTools>, OverrideError> {
    if cfg!(windows) {
        return Ok(None);
    }
    let Some(prefix) = game.proton_prefix.clone() else {
        return Ok(None);
    };
    if prefix_is_busy(&prefix) {
        return Err(OverrideError::PrefixBusy);
    }
    let config_info = std::fs::read_to_string(game.proton_compat_dir().join("config_info"))
        .map_err(|_| OverrideError::NoProton)?;
    let files_dir = find_proton_files_dir(&config_info).ok_or(OverrideError::NoProton)?;
    let wine = files_dir.join("bin/wine");
    if !wine.is_file() {
        return Err(OverrideError::NoProton);
    }
    Ok(Some(ProtonTools {
        wineserver: files_dir.join("bin/wineserver"),
        wine,
        prefix,
    }))
}

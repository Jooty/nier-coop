use std::path::PathBuf;
use std::process::Command;

use thiserror::Error;

use crate::install::PROXY_DLL_STEM;
use crate::proton::{
    DLL_OVERRIDE_VALUE, USER_REG_BACKUP_SUFFIX, find_proton_files_dir, prefix_is_busy,
};
use crate::steam::{EXE_NAME, GameInstall};

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

pub fn override_key() -> String {
    format!("HKCU\\Software\\Wine\\AppDefaults\\{EXE_NAME}\\DllOverrides")
}

pub fn override_is_set(reg_query_output: &str) -> bool {
    reg_query_output.contains(DLL_OVERRIDE_VALUE)
}

pub fn add_override(game: &GameInstall) -> Result<bool, OverrideError> {
    let Some(tools) = tools_for(game)? else {
        return Ok(false);
    };
    let current = tools
        .run_wine(&["reg", "query", &override_key(), "/v", PROXY_DLL_STEM])
        .unwrap_or_default();
    if override_is_set(&current) {
        return Ok(false);
    }
    tools.backup_user_reg()?;
    tools.run_wine(&[
        "reg",
        "add",
        &override_key(),
        "/v",
        PROXY_DLL_STEM,
        "/d",
        DLL_OVERRIDE_VALUE,
        "/f",
    ])?;
    tools.wait_for_wineserver();
    Ok(true)
}

pub fn remove_override(game: &GameInstall) -> Result<bool, OverrideError> {
    let Some(tools) = tools_for(game)? else {
        return Ok(false);
    };
    tools.backup_user_reg()?;
    let _ = tools.run_wine(&["reg", "delete", &override_key(), "/v", PROXY_DLL_STEM, "/f"]);
    tools.wait_for_wineserver();
    Ok(true)
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

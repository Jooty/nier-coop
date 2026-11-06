use std::path::PathBuf;

use crate::config::cache_dir;
use crate::install::{
    InstallOutcome, LEDGER_FILE_NAME, Ledger, fetch_dll_from_ci, install_dll, uninstall_dll,
};
use crate::proton::{add_override, remove_override};
use crate::steam::GameInstall;

#[derive(Debug, Clone, Default)]
pub struct ActionReport {
    pub lines: Vec<String>,
    pub failed: bool,
    pub fetched_dll: Option<PathBuf>,
}

impl ActionReport {
    fn line(&mut self, text: impl Into<String>) {
        self.lines.push(text.into());
    }

    fn fail(&mut self, text: impl Into<String>) {
        self.lines.push(text.into());
        self.failed = true;
    }
}

pub fn ledger_path() -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join(LEDGER_FILE_NAME))
}

pub fn run_install(game: GameInstall, dll_source: Option<PathBuf>) -> ActionReport {
    let mut report = ActionReport::default();
    let Some(ledger_file) = ledger_path() else {
        report.fail("no cache folder on this system");
        return report;
    };
    let source = match dll_source {
        Some(path) => path,
        None if !cfg!(debug_assertions) => {
            report.fail("rc_hook.dll was not found next to the launcher");
            return report;
        }
        None => match cache_dir()
            .ok_or_else(|| "no cache folder".to_owned())
            .and_then(|dir| fetch_dll_from_ci(&dir))
        {
            Ok(path) => {
                report.line(format!(
                    "downloaded the newest CI build to {}",
                    path.display()
                ));
                report.fetched_dll = Some(path.clone());
                path
            }
            Err(error) => {
                report.fail(format!(
                    "no rc_hook.dll next to the launcher and CI download failed: {error}"
                ));
                return report;
            }
        },
    };
    match install_dll(&game.game_dir, &source, &Ledger::new(&ledger_file)) {
        Ok(InstallOutcome::Installed) => report.line("installed the mod"),
        Ok(InstallOutcome::Updated) => report.line("updated the mod"),
        Ok(InstallOutcome::AlreadyCurrent) => {
            report.line("the mod is already installed and current")
        }
        Err(error) => {
            report.fail(format!("install failed: {error}"));
            return report;
        }
    }
    match add_override(&game) {
        Ok(true) => report.line("added the Proton DLL override"),
        Ok(false) => {}
        Err(error) => report.fail(format!("DLL override failed: {error}")),
    }
    report
}

pub fn run_uninstall(game: GameInstall) -> ActionReport {
    let mut report = ActionReport::default();
    let Some(ledger_file) = ledger_path() else {
        report.fail("no cache folder on this system");
        return report;
    };
    match uninstall_dll(&game.game_dir, &Ledger::new(&ledger_file)) {
        Ok(true) => report.line("removed the mod DLL"),
        Ok(false) => report.line("the mod DLL was not installed"),
        Err(error) => {
            report.fail(format!("uninstall failed: {error}"));
            return report;
        }
    }
    match remove_override(&game) {
        Ok(true) => report.line("removed the Proton DLL override"),
        Ok(false) => {}
        Err(error) => report.fail(format!("removing the DLL override failed: {error}")),
    }
    report
}

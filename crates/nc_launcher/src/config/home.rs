use std::ffi::OsStr;
use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use crate::config::{
    APP_DIR_NAME, CONFIG_FILE_NAME, LOCK_FILE_NAME, LOCK_RETRY_COUNT, LOCK_RETRY_PAUSE_MS,
    SECOND_LAUNCHER_DIR_NAME,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigRole {
    First,
    Second,
    Override,
}

#[derive(Debug)]
pub struct ConfigHome {
    pub dir: PathBuf,
    pub role: ConfigRole,
    pub seeded_from_main: bool,
    lock: Option<File>,
}

impl ConfigHome {
    pub fn holds_lock(&self) -> bool {
        self.lock.is_some()
    }
}

static PROCESS_HOME: OnceLock<Option<ConfigHome>> = OnceLock::new();

pub fn process_config_home() -> Option<&'static ConfigHome> {
    PROCESS_HOME
        .get_or_init(|| {
            open_config_home(
                std::env::var_os(crate::config::CONFIG_DIR_ENV).as_deref(),
                dirs::config_dir(),
                LOCK_RETRY_COUNT,
            )
        })
        .as_ref()
}

pub fn open_config_home(
    override_dir: Option<&OsStr>,
    platform_dir: Option<PathBuf>,
    lock_retries: u32,
) -> Option<ConfigHome> {
    if let Some(dir) = override_dir.filter(|dir| !dir.is_empty()) {
        return Some(ConfigHome {
            dir: PathBuf::from(dir),
            role: ConfigRole::Override,
            seeded_from_main: false,
            lock: None,
        });
    }
    let main_dir = platform_dir?.join(APP_DIR_NAME);
    match take_lock(&main_dir, lock_retries) {
        LockOutcome::Held(file) => Some(ConfigHome {
            dir: main_dir,
            role: ConfigRole::First,
            seeded_from_main: false,
            lock: Some(file),
        }),
        LockOutcome::Unavailable => Some(ConfigHome {
            dir: main_dir,
            role: ConfigRole::First,
            seeded_from_main: false,
            lock: None,
        }),
        LockOutcome::TakenByAnother => {
            let dir = main_dir.join(SECOND_LAUNCHER_DIR_NAME);
            let seeded_from_main = seed_from_main(&main_dir, &dir);
            Some(ConfigHome {
                dir,
                role: ConfigRole::Second,
                seeded_from_main,
                lock: None,
            })
        }
    }
}

enum LockOutcome {
    Held(File),
    TakenByAnother,
    Unavailable,
}

fn take_lock(dir: &Path, retries: u32) -> LockOutcome {
    if std::fs::create_dir_all(dir).is_err() {
        return LockOutcome::Unavailable;
    }
    let Ok(file) = File::options()
        .create(true)
        .write(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE_NAME))
    else {
        return LockOutcome::Unavailable;
    };
    for attempt in 0..=retries {
        match file.try_lock() {
            Ok(()) => return LockOutcome::Held(file),
            Err(TryLockError::WouldBlock) => {
                if attempt < retries {
                    std::thread::sleep(Duration::from_millis(LOCK_RETRY_PAUSE_MS));
                }
            }
            Err(TryLockError::Error(_)) => return LockOutcome::Unavailable,
        }
    }
    LockOutcome::TakenByAnother
}

fn seed_from_main(main_dir: &Path, second_dir: &Path) -> bool {
    let target = second_dir.join(CONFIG_FILE_NAME);
    if target.exists() || std::fs::create_dir_all(second_dir).is_err() {
        return false;
    }
    std::fs::copy(main_dir.join(CONFIG_FILE_NAME), target).is_ok()
}

use std::path::{Path, PathBuf};

use nc_launcher_core::GameProfile;

use crate::config::{LauncherConfig, cache_dir};
use crate::install::{
    Ledger, ModStatus, dll_next_to_executable, mod_status, newer_dll, newest_cached_ci_dll,
    sha256_file,
};
use crate::saves::{read_coop_slots, save_dir};
use crate::steam::{GameInstall, detect_game};

const SHORT_BUILD_CHARS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameCard {
    pub profile: &'static GameProfile,
    pub install: Option<GameInstall>,
    pub dll_source: Option<PathBuf>,
    pub mod_status: ModStatus,
    pub installed_build: Option<String>,
    pub coop_saves: Option<usize>,
}

pub fn build_card(
    profile: &'static GameProfile,
    config: &LauncherConfig,
    ledger_file: Option<&Path>,
) -> GameCard {
    let install = detect_game(profile, config);
    let dll_source = newer_dll(
        dll_next_to_executable(profile),
        cache_dir().and_then(|dir| newest_cached_ci_dll(profile, &dir)),
    );
    let status = match (&install, ledger_file) {
        (Some(game), Some(ledger)) => mod_status(
            profile,
            &game.game_dir,
            dll_source.as_deref(),
            &Ledger::new(ledger),
        ),
        _ => ModStatus::NotInstalled,
    };
    let installed_build = install
        .as_ref()
        .filter(|_| matches!(status, ModStatus::Current | ModStatus::Outdated))
        .and_then(|game| sha256_file(&game.game_dir.join(profile.loader_dll_name)).ok())
        .map(|hash| hash.chars().take(SHORT_BUILD_CHARS).collect());
    let coop_saves = install
        .as_ref()
        .filter(|_| profile.support.saves)
        .and_then(save_dir)
        .and_then(|dir| read_coop_slots(&dir).ok())
        .map(|slots| slots.iter().filter(|slot| slot.used).count());
    GameCard {
        profile,
        install,
        dll_source,
        mod_status: status,
        installed_build,
        coop_saves,
    }
}

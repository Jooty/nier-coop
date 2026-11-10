use std::sync::OnceLock;

use automata_game::build::SUPPORTED_PE_IDENTITIES;
use automata_game::save_redirect::{SaveBoot, SaveRedirectStatus};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::character_setting::character_from_launcher;
use crate::game_hooks::{install_save_redirect, log_second_copy_state};
use crate::start_setting::{GameSettings, settings_from_launcher};

static BOOT: OnceLock<Boot> = OnceLock::new();

#[derive(Debug)]
pub struct Boot {
    pub save_boot: SaveBoot,
    pub settings: GameSettings,
}

/// Runs the build check, the settings read and the save redirect exactly once, on whichever of
/// the init thread and the game's `DirectInput8Create` call comes first; the other one waits.
pub fn boot_once() -> &'static Boot {
    BOOT.get_or_init(run_boot)
}

fn run_boot() -> Boot {
    log_line(&character_from_launcher().log_line);
    let settings = settings_from_launcher();
    log_line(&settings.log_line);
    log_second_copy_state();
    let build = match VerifiedBuild::verify(&SUPPORTED_PE_IDENTITIES) {
        Ok(build) => build,
        Err(error) => {
            log_line(&format!("build gate: FAIL ({error}); nothing installed"));
            log_line(SaveRedirectStatus::NotRedirected.status_line());
            return Boot {
                save_boot: SaveBoot::UnknownExe,
                settings,
            };
        }
    };
    let save_boot = if install_save_redirect(build, settings.role) {
        SaveBoot::Redirected
    } else {
        SaveBoot::NotRedirected
    };
    Boot {
        save_boot,
        settings,
    }
}

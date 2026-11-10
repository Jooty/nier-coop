use std::sync::OnceLock;

use automata_game::build::{SUPPORTED_PE_IDENTITIES, variant_of_image_size};
use automata_game::save_redirect::SaveRedirectStatus;
use nc_hook::build_gate::{GateError, VerifiedBuild};
use nc_hook::diagnostics::log_line;
use nc_hook::overlay::{OverlayConfig, install_when_renderer_ready as install_overlay};

use crate::BUILD_COMMIT;
use crate::actions;
use crate::character_setting::character_from_launcher;
use crate::game_hooks::{
    install_input_block, install_save_redirect, log_second_copy_state, save_redirect_status,
};
use crate::menu::AutomataMenu;
use crate::start_driver;
use crate::start_setting::{GameSettings, settings_from_launcher};

const AUTOMATA_OVERLAY: OverlayConfig<AutomataMenu> = OverlayConfig {
    supported_builds: &SUPPORTED_PE_IDENTITIES,
    on_build_verified: start_automata_hooks,
    make_game: make_menu,
};

static GAME_SETTINGS: OnceLock<GameSettings> = OnceLock::new();

pub fn install_when_renderer_ready() {
    log_line(&character_from_launcher().log_line);
    let settings = GAME_SETTINGS.get_or_init(settings_from_launcher);
    log_line(&settings.log_line);
    log_second_copy_state();
    let build = match VerifiedBuild::verify(&SUPPORTED_PE_IDENTITIES) {
        Ok(build) => build,
        Err(error) => {
            log_line(&format!("build gate: FAIL ({error}); nothing installed"));
            log_line(SaveRedirectStatus::NotRedirected.status_line());
            return;
        }
    };
    install_save_redirect(build, settings.role);
    install_overlay(&AUTOMATA_OVERLAY);
}

fn start_automata_hooks(build: VerifiedBuild) {
    let variant = variant_of_image_size(build.size_of_image());
    log_line(&format!(
        "exe variant: {}",
        variant.map_or("unknown", |variant| variant.label())
    ));
    actions::install(build);
    let queued = actions::enqueue(actions::Action::LogLine(
        "queued action ran on the game thread".to_owned(),
    ));
    if let Err(error) = queued {
        log_line(&format!("actions: {error}"));
    }
    install_input_block(build);
    if let Some(settings) = GAME_SETTINGS.get() {
        start_driver::install(
            build,
            settings,
            save_redirect_status() == SaveRedirectStatus::CoopFolder,
        );
    }
}

fn make_menu(gate: &Result<VerifiedBuild, GateError>) -> AutomataMenu {
    let variant = gate
        .as_ref()
        .ok()
        .and_then(|build| variant_of_image_size(build.size_of_image()));
    AutomataMenu::new(variant, BUILD_COMMIT)
        .with_save_status(save_redirect_status)
        .with_start_status(start_driver::status_line)
}

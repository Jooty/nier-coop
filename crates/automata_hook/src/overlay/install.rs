use automata_game::build::{SUPPORTED_PE_IDENTITIES, variant_of_image_size};
use nc_hook::build_gate::{GateError, VerifiedBuild};
use nc_hook::diagnostics::log_line;
use nc_hook::overlay::{OverlayConfig, install_when_renderer_ready as install_overlay};

use crate::BUILD_COMMIT;
use crate::actions;
use crate::character_setting::character_from_launcher;
use crate::game_hooks::install_input_block;
use crate::menu::AutomataMenu;

const AUTOMATA_OVERLAY: OverlayConfig<AutomataMenu> = OverlayConfig {
    supported_builds: &SUPPORTED_PE_IDENTITIES,
    on_build_verified: start_automata_hooks,
    make_game: make_menu,
};

pub fn install_when_renderer_ready() {
    log_line(&character_from_launcher().log_line);
    if let Err(error) = VerifiedBuild::verify(&SUPPORTED_PE_IDENTITIES) {
        log_line(&format!("build gate: FAIL ({error}); nothing installed"));
        return;
    }
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
}

fn make_menu(gate: &Result<VerifiedBuild, GateError>) -> AutomataMenu {
    let variant = gate
        .as_ref()
        .ok()
        .and_then(|build| variant_of_image_size(build.size_of_image()));
    AutomataMenu::new(variant, BUILD_COMMIT)
}

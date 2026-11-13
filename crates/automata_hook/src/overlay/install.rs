use automata_game::build::{SUPPORTED_PE_IDENTITIES, variant_of_image_size};
use automata_game::save_redirect::{SaveBoot, SaveRedirectStatus};
use nc_hook::build_gate::{GateError, VerifiedBuild};
use nc_hook::diagnostics::log_line;
use nc_hook::overlay::{OverlayConfig, install_when_renderer_ready as install_overlay};

use crate::BUILD_COMMIT;
use crate::actions;
use crate::boot::boot_once;
use crate::enemies_driver;
use crate::game_hooks::{install_input_block, save_redirect_status, story_gate_status};
use crate::hud;
use crate::menu::{AutomataMenu, DownedHooks};
use crate::net::co_op_status;
use crate::partner_driver;
use crate::pickups_driver;
use crate::start_driver;
use crate::travel_driver;

const AUTOMATA_OVERLAY: OverlayConfig<AutomataMenu> = OverlayConfig {
    supported_builds: &SUPPORTED_PE_IDENTITIES,
    on_build_verified: start_automata_hooks,
    make_game: make_menu,
};

pub fn install_when_renderer_ready() {
    // Without the redirect the game is stopped at its DirectInput8Create call, so nothing more
    // is installed.
    if boot_once().save_boot == SaveBoot::Redirected {
        install_overlay(&AUTOMATA_OVERLAY);
    }
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
    let settings = &boot_once().settings;
    start_driver::install(
        build,
        settings,
        save_redirect_status() == SaveRedirectStatus::CoopFolder,
    );
    partner_driver::install(build, settings);
    travel_driver::install(build);
    pickups_driver::install(build);
    hud::install(build);
    enemies_driver::install(build, settings);
}

fn make_menu(gate: &Result<VerifiedBuild, GateError>) -> AutomataMenu {
    let variant = gate
        .as_ref()
        .ok()
        .and_then(|build| variant_of_image_size(build.size_of_image()));
    AutomataMenu::new(variant, BUILD_COMMIT)
        .with_save_status(save_redirect_status)
        .with_start_status(start_driver::status_line)
        .with_story_status(story_gate_status)
        .with_partner_body_status(partner_driver::body_status_line)
        .with_co_op_status(co_op_status)
        .with_teleport(travel_driver::teleport_hooks())
        .with_downed(DownedHooks {
            hero_is_down: partner_driver::hero_is_down,
            ask_to_give_up: partner_driver::ask_to_give_up,
        })
}

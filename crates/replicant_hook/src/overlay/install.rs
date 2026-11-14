use nt_hook::overlay::{OverlayConfig, install_when_renderer_ready as install_overlay};
use replicant_game::build::SUPPORTED_PE_IDENTITIES;

use crate::actions;
use crate::build_gate::VerifiedBuild;
use crate::game_hooks;
use crate::net;
use crate::overlay::replicant_overlay::ReplicantOverlay;

const REPLICANT_OVERLAY: OverlayConfig<ReplicantOverlay> = OverlayConfig {
    supported_builds: &SUPPORTED_PE_IDENTITIES,
    on_build_verified: start_replicant_hooks,
    make_game: ReplicantOverlay::new,
};

pub fn install_when_renderer_ready() {
    install_overlay(&REPLICANT_OVERLAY);
}

fn start_replicant_hooks(build: VerifiedBuild) {
    actions::install(build);
    game_hooks::install(build);
    net::start_auto();
}

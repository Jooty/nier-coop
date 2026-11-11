use std::sync::OnceLock;

use automata_game::build::variant_of_image_size;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

static HUD_CALLS: OnceLock<GameCalls> = OnceLock::new();

pub(crate) fn install(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("hud: image size matches no known exe, no partner tag, map dot or co-op lines");
        return;
    };
    let calls = GameCalls::new(build, variant);
    let _ = HUD_CALLS.set(calls);
    log_line(
        "hud: the partner's name tag reads the game camera's two matrices each frame (layout read from both exes, not seen in a game yet)",
    );
}

pub(super) fn calls() -> Option<&'static GameCalls> {
    HUD_CALLS.get()
}

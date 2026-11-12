use std::sync::OnceLock;

use automata_game::build::variant_of_image_size;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::hud::game_log_hook::install_game_log;
use crate::hud::minimap_hook::install_minimap_reader;

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
    log_line(if install_minimap_reader(&calls) {
        "minimap dot: reads the game's own minimap after each of its updates (read from both exes; the screen mapping is not seen in a game yet)"
    } else {
        "minimap dot: the game's minimap update is not hooked (its start differs), no dot is drawn"
    });
    log_line(if install_game_log(&calls) {
        "co-op log: lines go to the game's own HUD log (the call and the text lookup are read from both exes, not seen in a game yet)"
    } else {
        "co-op log: the game's own log is not hooked, lines are drawn by the overlay"
    });
}

pub(super) fn calls() -> Option<&'static GameCalls> {
    HUD_CALLS.get()
}

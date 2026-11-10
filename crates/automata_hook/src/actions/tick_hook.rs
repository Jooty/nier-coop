use std::sync::atomic::{AtomicBool, Ordering};

use automata_game::address::FRAME_END_FUNCTION;
use automata_game::build::variant_of_image_size;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::tick_hook::install_game_thread_tick_with_result;

use crate::actions::queue::take_all;

static FIRST_TICK_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn install(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("tick: image size matches no known exe, tick hook not installed");
        return;
    };
    install_game_thread_tick_with_result(build, FRAME_END_FUNCTION.rva(variant), run_game_tick);
}

fn run_game_tick(_build: &VerifiedBuild) {
    if !FIRST_TICK_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(
            "tick: first frame-end call seen on the game thread (hook point is a guess until this line appears)",
        );
    }
    for action in take_all() {
        action.run();
    }
    crate::start_driver::tick();
    crate::partner_driver::tick();
}

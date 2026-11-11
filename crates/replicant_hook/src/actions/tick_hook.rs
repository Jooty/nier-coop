use nc_hook::tick_hook::install_game_thread_tick;
use replicant_game::offsets::SCRIPT_MANAGER_TICK_RVA;

use crate::actions::runner::run_queued_actions;
use crate::blocks::tick_blocks;
use crate::bombs::tick_walls;
use crate::boot::tick_first_save;
use crate::build_gate::VerifiedBuild;
use crate::desync::tick_desync;
use crate::enemies::tick_enemy_sync;
use crate::gimmick_probe::tick_gimmick_probe;
use crate::move_log::tick_move_log;
use crate::net;
use crate::npcs::tick_npc_sync;
use crate::sky::tick_sky_sync;

pub use nc_hook::tick_hook::is_draining;

pub fn install(build: VerifiedBuild) {
    install_game_thread_tick(build, SCRIPT_MANAGER_TICK_RVA, run_game_tick);
}

fn run_game_tick(build: &VerifiedBuild) {
    run_queued_actions(build);
    net::quit_when_asked();
    net::report_readiness(build);
    net::publish_local_snapshot(build);
    tick_enemy_sync(build);
    tick_npc_sync(build);
    tick_sky_sync(build);
    tick_desync(build);
    tick_move_log(build);
    tick_gimmick_probe(build);
    tick_blocks(build);
    tick_walls(build);
    tick_first_save(build);
}

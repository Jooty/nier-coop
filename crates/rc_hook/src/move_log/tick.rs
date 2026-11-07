use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::marker::read_world_flags;
use rc_game::move_log::{
    MoveLineContext, MoveLogTrigger, MoveLogWindow, action_set_for_seed, format_move_line,
    read_move_sample,
};
use rc_game::puppet::body_info;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::window_unfocused;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::player_body::real_body;

const UNKNOWN_SEED: &str = "unknown";

static STATE: LazyLock<Mutex<(MoveLogTrigger, MoveLogWindow)>> = LazyLock::new(Mutex::default);

pub fn tick_move_log(build: &VerifiedBuild) {
    let game = Replicant::new(InProcessMemory::new(build));
    let actor = game.playable_actor().unwrap_or(0);
    let flags = read_world_flags(game.memory()).ok();
    let now = net_clock_ms();
    let mut state = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let (trigger, window) = &mut *state;
    let mask = flags.map_or(0, |flags| flags.control_mask);
    let talk_open = flags.is_some_and(|flags| flags.talk_window_open);
    if let Some(reason) = trigger.observe(actor, mask, talk_open) {
        window.arm(reason, now);
    }
    if actor == 0 {
        return;
    }
    let Some(due) = window.on_tick(now) else {
        return;
    };
    let seed = real_body().map_or(UNKNOWN_SEED, |real| body_info(real.body).own_seed_name);
    let context = MoveLineContext {
        due,
        action_set: action_set_for_seed(seed),
        seed,
        talk_window_open: talk_open,
        window_focused: !window_unfocused(),
    };
    log_line(&format_move_line(
        &context,
        &read_move_sample(game.memory(), actor),
    ));
}

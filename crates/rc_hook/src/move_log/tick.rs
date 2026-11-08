use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::marker::read_world_flags;
use rc_game::move_log::{
    MoveLineContext, MoveLogTrigger, MoveLogWindow, SpecDumpSchedule, action_set_for_seed,
    format_move_line, format_rates_dump, format_spec_dump, input_block_getter, read_move_axis, read_move_sample,
    read_rates_dump, read_spec_table, role_label,
};
use rc_game::puppet::body_info;
use rc_game::stay_together::FollowRole;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{game_window_is_foreground, take_focus_event_counts, window_unfocused};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::player_body::{lift_house_speed_cap, real_body};

const UNKNOWN_SEED: &str = "unknown";

static STATE: LazyLock<Mutex<(MoveLogTrigger, MoveLogWindow)>> = LazyLock::new(Mutex::default);
static SPEC_DUMP_SCHEDULE: LazyLock<Mutex<SpecDumpSchedule>> = LazyLock::new(Mutex::default);

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
    log_spec_dump_when_due(&game, actor, now);
    if actor == 0 {
        return;
    }
    lift_house_speed_cap(
        build,
        actor,
        flags.is_some_and(|flags| flags.event_scene),
        talk_open,
    );
    let Some(due) = window.on_tick(now) else {
        return;
    };
    let seed = real_body().map_or(UNKNOWN_SEED, |real| body_info(real.body).own_seed_name);
    let (focus_losses, key_clears) = take_focus_event_counts();
    let context = MoveLineContext {
        due,
        action_set: action_set_for_seed(seed),
        seed,
        talk_window_open: talk_open,
        window_focused: !window_unfocused(),
        window_foreground: game_window_is_foreground(),
        focus_losses,
        key_clears,
    };
    let mut sample = read_move_sample(game.memory(), actor);
    sample.move_axis = game_move_axis(&game, actor);
    log_line(&format_move_line(&context, &sample));
}

fn log_spec_dump_when_due(game: &Replicant<InProcessMemory>, actor: u64, now: u64) {
    let phase_loads = real_body().map_or(0, |real| real.phase_loads);
    let moment = SPEC_DUMP_SCHEDULE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(actor, phase_loads, now);
    let Some(moment) = moment else {
        return;
    };
    let seed = real_body().map_or(UNKNOWN_SEED, |real| body_info(real.body).own_seed_name);
    let role = net::handle().map_or(FollowRole::Unknown, |net| {
        FollowRole::from_slot(net.view().local_slot)
    });
    let table = read_spec_table(game.memory(), actor);
    for line in format_spec_dump(
        role_label(role),
        action_set_for_seed(seed),
        seed,
        moment,
        table.as_ref(),
    ) {
        log_line(&line);
    }
    let rates = read_rates_dump(game.memory(), actor);
    for line in format_rates_dump(role_label(role), moment.label(), &rates) {
        log_line(&line);
    }
}

fn game_move_axis(game: &Replicant<InProcessMemory>, actor: u64) -> Option<[f32; 2]> {
    let getter = input_block_getter(game.memory(), actor)?;
    #[expect(
        unsafe_code,
        reason = "calls the player actor's own input block getter, a plain virtual that returns a pointer"
    )]
    // SAFETY: the slot was read from the actor's vftable and lies inside the game's code; the
    // getter takes the actor and returns a pointer into the actor's input block.
    let block = unsafe {
        core::mem::transmute::<u64, unsafe extern "system" fn(u64) -> u64>(getter)(actor)
    };
    if block == 0 {
        return None;
    }
    read_move_axis(game.memory(), block)
}

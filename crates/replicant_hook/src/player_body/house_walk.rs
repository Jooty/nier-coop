use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::move_log::{
    SpeedCapLift, SpeedCapReading, SpeedCapSecond, SpeedCapWatch, format_speed_cap_log,
    format_speed_cap_second, read_move_sample, read_speed_cap, read_state_rva,
    write_lifted_speed_cap,
};
use replicant_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::player_body::real_body;

static SPEED_CAP_LIFT: LazyLock<Mutex<(SpeedCapLift, SpeedCapWatch)>> =
    LazyLock::new(Mutex::default);

pub fn lift_house_speed_cap(
    build: &VerifiedBuild,
    actor: u64,
    event_scene: bool,
    talk: bool,
    control_masked: bool,
) {
    let real = real_body();
    let kaine_swapped =
        real.is_some_and(|real| real.by_swap && real.body == PartnerBody::SeedKaineE);
    let phase = real.map_or(0, |real| real.phase_loads);
    let mut memory = InProcessMemory::new(build);
    let Some(value) = read_speed_cap(&memory, actor) else {
        return;
    };
    let now_ms = net_clock_ms();
    let mut state = SPEED_CAP_LIFT
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let (lift, watch) = &mut *state;
    let step = lift.observe(SpeedCapReading {
        phase,
        now_ms,
        value,
        kaine_swapped,
        event_scene,
        talk,
        control_masked,
        state_rva: read_state_rva(&memory, actor),
    });
    if step.write && !write_lifted_speed_cap(&mut memory, actor) {
        log_line("kaine walk: cannot write the speed cap");
    }
    if let Some(log) = step.log {
        log_line(&format_speed_cap_log(log, phase));
    }
    if !kaine_swapped {
        return;
    }
    let Some(seconds_after_load) = watch.due(phase, now_ms) else {
        return;
    };
    let sample = read_move_sample(&memory, actor);
    log_line(&format_speed_cap_second(&SpeedCapSecond {
        phase,
        seconds_after_load,
        cap: read_speed_cap(&memory, actor),
        lifts_this_phase: lift.lifts_this_phase(),
        step_per_second: sample.applied_step_per_second,
        ground_speed: sample.speed,
        state_rva: sample.state_rva,
    }));
}

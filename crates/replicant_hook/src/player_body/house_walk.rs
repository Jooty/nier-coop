use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::move_log::{
    SpeedCapLift, format_speed_cap_log, read_speed_cap, write_lifted_speed_cap,
};
use replicant_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::player_body::real_body;

static SPEED_CAP_LIFT: LazyLock<Mutex<SpeedCapLift>> = LazyLock::new(Mutex::default);

pub fn lift_house_speed_cap(build: &VerifiedBuild, actor: u64, event_scene: bool, talk: bool) {
    let real = real_body();
    let kaine_swapped =
        real.is_some_and(|real| real.by_swap && real.body == PartnerBody::SeedKaineE);
    let phase = real.map_or(0, |real| real.phase_loads);
    let mut memory = InProcessMemory::new(build);
    let Some(value) = read_speed_cap(&memory, actor) else {
        return;
    };
    let step = SPEED_CAP_LIFT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(phase, value, kaine_swapped, event_scene, talk);
    if step.write && !write_lifted_speed_cap(&mut memory, actor) {
        log_line("kaine walk: cannot write the speed cap");
    }
    if let Some(log) = step.log {
        log_line(&format_speed_cap_log(log, phase));
    }
}

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::GameMemory;
use replicant_game::kaine::{KAINE_LEVEL_SAVE_OFFSET, sync_kaine_stats};
use replicant_game::offsets::{ENTITY_LEVEL_OFFSET, ENTITY_RVA};
use replicant_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::player_body::real_body;

const STATS_SYNC_INTERVAL: Duration = Duration::from_secs(2);

static LAST_SYNC: Mutex<Option<(u32, Instant)>> = Mutex::new(None);

pub(crate) fn match_kaine_stats_to_protagonist(build: &VerifiedBuild) {
    let Some(real) = real_body() else {
        return;
    };
    if !(real.by_swap && real.body == PartnerBody::SeedKaineE) {
        return;
    }
    let now = Instant::now();
    let mut last = LAST_SYNC.lock().unwrap_or_else(PoisonError::into_inner);
    let due = last.is_none_or(|(phase, at)| {
        phase != real.phase_loads || now.saturating_duration_since(at) >= STATS_SYNC_INTERVAL
    });
    if !due {
        return;
    }
    *last = Some((real.phase_loads, now));
    let mut memory = InProcessMemory::new(build);
    match sync_kaine_stats(&mut memory) {
        Ok(Some(synced)) => log_line(&synced.log_line()),
        Ok(None) => {}
        Err(error) => log_line(&format!("kaine: cannot match her stats ({error})")),
    }
}

pub(crate) fn set_her_level_before_player_is_built(build: &VerifiedBuild) {
    let Some(real) = real_body() else {
        return;
    };
    if !(real.by_swap && real.body == PartnerBody::SeedKaineE) {
        return;
    }
    let mut memory = InProcessMemory::new(build);
    let save = memory.module_base() + u64::from(ENTITY_RVA);
    let her_level_at = save + KAINE_LEVEL_SAVE_OFFSET as u64;
    let (Ok(own), Ok(hers)) = (
        memory.read_i32(save + u64::from(ENTITY_LEVEL_OFFSET)),
        memory.read_i32(her_level_at),
    ) else {
        log_line("kaine stats: cannot read the levels before the player is built");
        return;
    };
    if hers != own && memory.write_i32(her_level_at, own).is_err() {
        log_line("kaine stats: cannot write her level before the player is built");
        return;
    }
    log_line(&format!(
        "kaine stats: level set from {hers} to {own} before the player is built (phase {})",
        real.phase_loads
    ));
}

use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rc_game::enemies::{
    CA_GAUGE_ENTRY_COUNT, CA_GAUGE_SCAN_INTERVAL_MS, CaGaugeEntry, CaGaugeEvent, CaGaugeWatch,
    EnemySighting,
};
use rc_proto::EnemyId;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::memory::InProcessMemory;

struct ProbeState {
    watch: CaGaugeWatch,
    last_scan_ms: Option<u64>,
}

static PROBE: LazyLock<Mutex<ProbeState>> = LazyLock::new(|| {
    Mutex::new(ProbeState {
        watch: CaGaugeWatch::new(),
        last_scan_ms: None,
    })
});
static PROBE_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(super) fn log_ca_gauge_entries(
    build: &VerifiedBuild,
    seen: &[(EnemyId, EnemySighting)],
) {
    let now_ms = u64::try_from(PROBE_CLOCK.elapsed().as_millis()).unwrap_or(u64::MAX);
    let mut probe = PROBE.lock().unwrap_or_else(PoisonError::into_inner);
    if probe
        .last_scan_ms
        .is_some_and(|last| now_ms.saturating_sub(last) < CA_GAUGE_SCAN_INTERVAL_MS)
    {
        return;
    }
    probe.last_scan_ms = Some(now_ms);
    let memory = InProcessMemory::new(build);
    for index in 0..CA_GAUGE_ENTRY_COUNT {
        let Some(entry) = CaGaugeEntry::read(&memory, index) else {
            continue;
        };
        let Some(event) = probe.watch.observe(now_ms, index, &entry) else {
            continue;
        };
        log_line(&event_text(&memory, seen, &event));
    }
}

fn event_text(
    memory: &InProcessMemory,
    seen: &[(EnemyId, EnemySighting)],
    event: &CaGaugeEvent,
) -> String {
    match event {
        CaGaugeEvent::Opened { index, handle } => {
            let enemy = handle.resolve(memory).and_then(|actor| {
                seen.iter()
                    .find(|(_, sighting)| sighting.actor == actor)
                    .map(|(id, _)| enemy_id_text(*id))
            });
            format!(
                "ca gauge: entry {index} opened, handle {}/{} (enemy {})",
                handle.index,
                handle.generation,
                enemy.unwrap_or_else(|| "?".to_owned())
            )
        }
        CaGaugeEvent::Closed { index } => format!("ca gauge: entry {index} closed"),
        CaGaugeEvent::Changed { index, entry } => format!(
            "ca gauge: entry {index} +0x4C {}, +0x50 {}, raw {}",
            entry.value,
            entry.timer,
            entry.raw_hex()
        ),
    }
}

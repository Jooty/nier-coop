use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use replicant_game::enemies::{
    CA_GAUGE_ENTRY_COUNT, CA_GAUGE_HEARTBEAT_INTERVAL_MS, CA_GAUGE_SCAN_INTERVAL_MS, CaGaugeEntry,
    CaGaugeEvent, CaGaugeScanStats, CaGaugeWatch, EnemySighting, IntervalClock,
    ca_gauge_heartbeat_text, ca_gauge_probe_on_text, read_ca_gauge_manager_raw,
};
use replicant_proto::EnemyId;

use crate::boss_messages::is_boss_fight_on;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::memory::InProcessMemory;
use replicant_game::GameMemory;

struct ProbeState {
    watch: CaGaugeWatch,
    last_scan_ms: Option<u64>,
    heartbeat: IntervalClock,
    announced: bool,
    read_failures: u64,
}

static PROBE: LazyLock<Mutex<ProbeState>> = LazyLock::new(|| {
    Mutex::new(ProbeState {
        watch: CaGaugeWatch::new(),
        last_scan_ms: None,
        heartbeat: IntervalClock::new(),
        announced: false,
        read_failures: 0,
    })
});
static PROBE_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);

pub(super) fn log_ca_gauge_entries(build: &VerifiedBuild, seen: &[(EnemyId, EnemySighting)]) {
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
    if !probe.announced {
        probe.announced = true;
        log_line(&ca_gauge_probe_on_text(memory.module_base()));
    }
    let mut in_use = 0;
    let mut first_read_ok = false;
    for index in 0..CA_GAUGE_ENTRY_COUNT {
        let Some(entry) = CaGaugeEntry::read(&memory, index) else {
            probe.read_failures += 1;
            continue;
        };
        first_read_ok |= index == 0;
        in_use += usize::from(entry.in_use);
        let Some(event) = probe.watch.observe(now_ms, index, &entry) else {
            continue;
        };
        log_line(&event_text(&memory, seen, &event));
    }
    if !is_boss_fight_on() {
        probe.heartbeat.reset();
        return;
    }
    if probe.heartbeat.due(now_ms, CA_GAUGE_HEARTBEAT_INTERVAL_MS) {
        let stats = CaGaugeScanStats {
            in_use,
            first_read_ok,
            read_failures: probe.read_failures,
        };
        log_line(&ca_gauge_heartbeat_text(
            &stats,
            read_ca_gauge_manager_raw(&memory).as_ref(),
        ));
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

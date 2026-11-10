use std::sync::PoisonError;

use rc_game::Replicant;
use rc_game::attack_gauge::{
    ATTACK_GAUGE_SWITCHED_OFF_TEXT, GaugeHitFacts, GaugeHitVerdict, GaugeReading, gauge_close_text,
    gauge_hit_applied_text, gauge_hit_dropped_text, gauge_open_text, read_gauge,
    read_hp_floor_percent, write_node_hp,
};
use rc_game::enemies::EnemySighting;
use rc_proto::{AttackGaugeHit, AttackGaugeOpen, EnemyId, Message};

use crate::attack_gauge::calls::GaugeCalls;
use crate::attack_gauge::state::{HOST, HostSide, now_ms, off_file_present};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::NetThread;

/// The host's gauge step, on the game thread after each enemy scan: applies the guest's gauge hits
/// to the host's own gauge, then sends the gauge's state when it opened, changed or closed.
pub fn tick_host(
    build: &VerifiedBuild,
    net: &NetThread,
    phase: u16,
    seen: &[(EnemyId, EnemySighting)],
) {
    let hits: Vec<AttackGaugeHit> = net
        .take_relayed(|message| matches!(message, Message::AttackGaugeHit(_)))
        .into_iter()
        .filter_map(|message| match message {
            Message::AttackGaugeHit(hit) => Some(hit),
            _ => None,
        })
        .collect();
    let game = Replicant::new(InProcessMemory::new(build));
    let mut host = HOST.lock().unwrap_or_else(PoisonError::into_inner);
    let gauge_open = read_gauge(game.memory()).is_some();
    if host.switch.observe(now_ms(), gauge_open, off_file_present) {
        if host.switch.should_log_off(phase) {
            log_line(ATTACK_GAUGE_SWITCHED_OFF_TEXT);
        }
        host.watch.reset();
        host.last_open = None;
        return;
    }
    if !hits.is_empty() {
        apply_guest_hits(build, &game, &mut host, phase, &hits);
    }
    let reading = read_gauge(game.memory());
    let open = reading
        .as_ref()
        .and_then(|reading| shareable_open(&mut host, reading, seen));
    let serial_before = host.watch.serial();
    let Some(state) = host.watch.observe(now_ms(), phase, open) else {
        return;
    };
    match &state.open {
        Some(open) => {
            if serial_before != Some(state.serial) {
                log_line(&gauge_open_text(state.serial, open));
            }
            host.last_open = Some(open.clone());
        }
        None => {
            log_line(&gauge_close_text(state.serial, host.last_open.as_ref()));
            host.last_open = None;
            host.hits.clear();
        }
    }
    net.send_relayed(Message::AttackGauge(state));
}

fn shareable_open(
    host: &mut HostSide,
    reading: &GaugeReading,
    seen: &[(EnemyId, EnemySighting)],
) -> Option<AttackGaugeOpen> {
    let id_of = |actor: u64| {
        seen.iter()
            .find(|(_, sighting)| sighting.actor == actor)
            .map(|(id, _)| *id)
    };
    let Some(boss) = id_of(reading.boss) else {
        if host.unshared_boss_logged != Some(reading.boss) {
            host.unshared_boss_logged = Some(reading.boss);
            log_line(&format!(
                "attack gauge: the gauge's boss {:#x} is not a shared enemy, not sent",
                reading.boss
            ));
        }
        return None;
    };
    let partner = match reading.partner {
        None => None,
        Some(actor) => Some(id_of(actor).or_else(|| {
            log_line(&format!(
                "attack gauge: the gauge's second actor {actor:#x} is not a shared enemy, not sent"
            ));
            None
        })?),
    };
    if reading.more_nodes && !host.long_list_logged {
        host.long_list_logged = true;
        log_line("attack gauge: the gauge has more nodes than are sent, only the first are shared");
    }
    Some(AttackGaugeOpen {
        boss,
        partner,
        total_seconds: reading.total_seconds,
        remaining_seconds: reading.remaining_seconds,
        map_points: reading.map_points,
        options: reading.options,
        nodes: reading.nodes.iter().map(|node| node.node).collect(),
    })
}

fn apply_guest_hits(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    host: &mut HostSide,
    phase: u16,
    hits: &[AttackGaugeHit],
) {
    let mut memory = InProcessMemory::new(build);
    let floor_percent = read_hp_floor_percent(&memory).unwrap_or(0);
    for hit in hits {
        let reading = read_gauge(&memory);
        let node = reading
            .as_ref()
            .and_then(|reading| reading.nodes.get(usize::from(hit.node)).copied());
        let facts = GaugeHitFacts {
            local_phase: phase,
            serial: reading.as_ref().and(host.watch.serial()),
            node_hp: node.map(|node| (node.node.hp, node.node.max_hp)),
            floor_percent,
            switched_off: host.switch.is_off(),
        };
        let verdict = host.hits.judge(&facts, hit);
        let GaugeHitVerdict::Apply {
            hp_before,
            hp_after,
            breaks,
        } = verdict
        else {
            if let GaugeHitVerdict::Drop(drop) = verdict {
                log_line(&gauge_hit_dropped_text(hit, drop));
            }
            continue;
        };
        let (Some(reading), Some(node)) = (reading, node) else {
            continue;
        };
        if !write_node_hp(&mut memory, &node, hp_after) {
            log_line(&format!(
                "attack gauge: cannot write node {} hp at {:#x}",
                hit.node, node.address
            ));
            continue;
        }
        log_line(&gauge_hit_applied_text(hit, hp_before, hp_after, breaks));
        if breaks {
            break_node(build, game, &reading, hit.node);
        }
    }
}

fn break_node(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    reading: &GaugeReading,
    node: u8,
) {
    let Ok(player) = game.playable_actor() else {
        log_line(
            "attack gauge: node broke but your player is not readable, the game's break did not run",
        );
        return;
    };
    GaugeCalls::new(build).break_node(reading.dummy, i32::from(node), player);
    let after = read_gauge(game.memory());
    log_line(&format!(
        "attack gauge: ran the game's node break for node {node}, the gauge now reads {}",
        if after.is_some() { "open" } else { "closed" }
    ));
}

use std::sync::PoisonError;

use replicant_game::Replicant;
use replicant_game::attack_gauge::{
    ATTACK_GAUGE_SWITCHED_OFF_TEXT, GaugeAskFacts, GaugeAskVerdict, GaugeHitFacts, GaugeHitVerdict,
    GaugeReading, GaugeSwingMiss, count_nodes, gauge_close_text, gauge_hit_applied_text,
    gauge_hit_dropped_text, gauge_open_text, gauge_tally_text, host_ask_opened_text,
    host_ask_refused_text, read_gauge, read_hp_floor_percent, write_gauge_clock, write_node_hp,
};
use replicant_game::enemies::EnemySighting;
use replicant_proto::{
    AttackGaugeAsk, AttackGaugeHit, AttackGaugeNode, AttackGaugeOpen, EnemyId, Message,
};

use crate::attack_gauge::calls::GaugeCalls;
use crate::attack_gauge::state::{HOST, HostSide, now_ms, off_file_present};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::NetThread;

/// The host's gauge step, on the game thread after each enemy scan: opens a gauge the guest's game
/// opened when the host has none, applies the guest's gauge hits to the host's own gauge, then
/// sends the gauge's state when it opened, changed or closed.
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
    let asks: Vec<AttackGaugeAsk> = net
        .take_relayed(|message| matches!(message, Message::AttackGaugeAsk(_)))
        .into_iter()
        .filter_map(|message| match message {
            Message::AttackGaugeAsk(ask) => Some(ask),
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
    for ask in &asks {
        answer_ask(build, &mut host, phase, seen, ask);
    }
    if !hits.is_empty() {
        apply_guest_hits(build, &game, &mut host, phase, &hits);
    }
    let reading = read_gauge(game.memory());
    let open = reading
        .as_ref()
        .and_then(|reading| shareable_open(&mut host, reading, seen));
    let serial_before = host.watch.serial();
    let now = now_ms();
    let observed = host.watch.observe(now, phase, open);
    if let Some(state) = observed {
        match &state.open {
            Some(open) => {
                if serial_before != Some(state.serial) {
                    log_line(&gauge_open_text(state.serial, open));
                    host.tally.reset();
                }
                host.last_open = Some(open.clone());
            }
            None => {
                log_line(&gauge_close_text(state.serial, host.last_open.as_ref()));
                host.last_open = None;
                host.hits.clear();
                host.last_close = Some((phase, now));
            }
        }
        net.send_relayed(Message::AttackGauge(state));
    }
    if let Some(reading) = &reading
        && host.watch.serial().is_some()
        && host.tally.line_due(now)
    {
        let nodes: Vec<AttackGaugeNode> = reading.nodes.iter().map(|node| node.node).collect();
        log_line(&gauge_tally_text(
            "host",
            host.watch.serial(),
            &nodes,
            &host.tally,
        ));
    }
}

fn answer_ask(
    build: &VerifiedBuild,
    host: &mut HostSide,
    phase: u16,
    seen: &[(EnemyId, EnemySighting)],
    ask: &AttackGaugeAsk,
) {
    let actor_of = |id: EnemyId| {
        seen.iter()
            .find(|(seen_id, _)| *seen_id == id)
            .map(|(_, sighting)| sighting.actor)
    };
    let mut memory = InProcessMemory::new(build);
    let boss = actor_of(ask.open.boss);
    let partner = ask.open.partner.and_then(actor_of);
    let now = now_ms();
    let facts = GaugeAskFacts {
        local_phase: phase,
        switched_off: host.switch.is_off(),
        host_gauge_open: read_gauge(&memory).is_some(),
        boss_found: boss.is_some(),
        partner_found: partner.is_some(),
        ms_since_host_close: host
            .last_close
            .filter(|(closed_phase, _)| *closed_phase == phase)
            .map(|(_, at)| now.saturating_sub(at)),
    };
    let verdict = host.asks.judge(&facts, ask);
    let (GaugeAskVerdict::Open, Some(boss)) = (verdict, boss) else {
        if let GaugeAskVerdict::Refuse(refusal) = verdict {
            log_line(&host_ask_refused_text(ask, refusal));
        }
        return;
    };
    let opened = open_for_guest(build, &mut memory, boss, partner, &ask.open);
    log_line(&host_ask_opened_text(ask, opened));
}

fn open_for_guest(
    build: &VerifiedBuild,
    memory: &mut InProcessMemory,
    boss: u64,
    partner: Option<u64>,
    open: &AttackGaugeOpen,
) -> Result<(), &'static str> {
    if count_nodes(memory) != Some(0) {
        return Err("your gauge list is not empty or not readable");
    }
    let calls = GaugeCalls::new(build);
    for (expected, node) in (0_i32..).zip(&open.nodes) {
        if calls.add_node(node) != expected {
            calls.close();
            return Err("a node came back at another index");
        }
    }
    if !calls.open(boss, partner, open) {
        calls.close();
        return Err("the game's open refused");
    }
    if read_gauge(memory).is_none() {
        calls.close();
        return Err("the gauge reads closed after opening");
    }
    if !write_gauge_clock(memory, open.total_seconds, open.remaining_seconds) {
        return Err("opened, but the clock could not be written");
    }
    Ok(())
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
        host.tally.sent = host.tally.sent.saturating_add(1);
        let verdict = host.hits.judge(&facts, hit);
        let GaugeHitVerdict::Apply {
            hp_before,
            hp_after,
            breaks,
        } = verdict
        else {
            if let GaugeHitVerdict::Drop(drop) = verdict {
                host.tally.miss(GaugeSwingMiss::DroppedByHost);
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
        host.tally.applied = host.tally.applied.saturating_add(1);
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

use replicant_proto::{AttackGaugeHit, AttackGaugeNode, AttackGaugeOpen, EnemyId};

use crate::attack_gauge::{GaugeHitDrop, GaugeReading, GuestGaugeStep, GuestGaugeWait};

pub fn gauge_open_text(serial: u32, open: &AttackGaugeOpen) -> String {
    format!(
        "attack gauge: host opened number {serial} on {}, {} nodes (hp {}), {:.1} s, sent",
        actors_text(open.boss, open.partner),
        open.nodes.len(),
        hp_list(&open.nodes),
        open.total_seconds
    )
}

pub fn gauge_close_text(serial: u32, last: Option<&AttackGaugeOpen>) -> String {
    last.map_or_else(
        || format!("attack gauge: host closed number {serial}, sent"),
        |open| {
            format!(
                "attack gauge: host closed number {serial} with {:.1} s left, nodes hp {}, sent",
                open.remaining_seconds,
                hp_list(&open.nodes)
            )
        },
    )
}

pub fn gauge_hit_applied_text(
    hit: &AttackGaugeHit,
    hp_before: i32,
    hp_after: i32,
    breaks: bool,
) -> String {
    format!(
        "attack gauge: guest hit number {} on node {} of number {} took {}, hp {hp_before} -> {hp_after}{}",
        hit.seq,
        hit.node,
        hit.serial,
        hit.damage,
        if breaks { ", the node broke" } else { "" }
    )
}

pub fn gauge_hit_dropped_text(hit: &AttackGaugeHit, drop: GaugeHitDrop) -> String {
    let reason = match drop {
        GaugeHitDrop::OtherPhase => "another phase",
        GaugeHitDrop::Closed => "the host's gauge is closed",
        GaugeHitDrop::OtherOpening => "another opening",
        GaugeHitDrop::NoSuchNode => "no such node",
        GaugeHitDrop::NodeAlreadyBroken => "the node is already broken",
        GaugeHitDrop::Repeated => "already applied",
    };
    format!(
        "attack gauge: guest hit number {} on node {} of number {} dropped ({reason})",
        hit.seq, hit.node, hit.serial
    )
}

pub fn guest_open_text(serial: u32, open: &AttackGaugeOpen) -> String {
    format!(
        "attack gauge: opened number {serial} on your copy of {} like the host's ({} nodes, {:.1} of {:.1} s)",
        actors_text(open.boss, open.partner),
        open.nodes.len(),
        open.remaining_seconds,
        open.total_seconds
    )
}

pub fn guest_open_readback_text(serial: u32, reading: Option<&GaugeReading>) -> String {
    let Some(reading) = reading else {
        return format!("attack gauge: after opening number {serial} the gauge reads closed");
    };
    let nodes: Vec<AttackGaugeNode> = reading.nodes.iter().map(|node| node.node).collect();
    format!(
        "attack gauge: after opening number {serial}: mode {}, dummy {:#x}, nodes {} (hp {}), {:.1} of {:.1} s",
        reading.mode,
        reading.dummy,
        nodes.len(),
        hp_list(&nodes),
        reading.remaining_seconds,
        reading.total_seconds
    )
}

pub fn guest_close_text(serial: Option<u32>, step: GuestGaugeStep) -> String {
    let reason = match step {
        GuestGaugeStep::CloseHostClosed => "the host's closed",
        GuestGaugeStep::CloseOtherOpening => "the host shows another opening",
        GuestGaugeStep::CloseHostHasNone => "the host has none",
        _ => "asked",
    };
    serial.map_or_else(
        || format!("attack gauge: closed your own gauge ({reason})"),
        |serial| format!("attack gauge: closed number {serial} ({reason})"),
    )
}

pub fn guest_wait_text(serial: Option<u32>, wait: GuestGaugeWait) -> String {
    let reason = match wait {
        GuestGaugeWait::BossCopyMissing => "your copy of the boss is not here yet",
        GuestGaugeWait::PartnerCopyMissing => "your copy of the second actor is not here yet",
        GuestGaugeWait::GuestOnlyGrace => "your own gauge opened, waiting for the host's",
    };
    serial.map_or_else(
        || format!("attack gauge: waiting ({reason})"),
        |serial| format!("attack gauge: waiting with number {serial} ({reason})"),
    )
}

pub fn own_gauge_hit_text(node: u8, damage: i32, seq: u32, serial: u32) -> String {
    format!(
        "attack gauge: your hit on node {node} took {damage}, sent to the host as number {seq} for opening {serial}"
    )
}

fn actors_text(boss: EnemyId, partner: Option<EnemyId>) -> String {
    let id = |id: EnemyId| format!("{}:{}", id.group, id.ordinal);
    partner.map_or_else(
        || id(boss),
        |partner| format!("{} and {}", id(boss), id(partner)),
    )
}

fn hp_list(nodes: &[AttackGaugeNode]) -> String {
    let parts: Vec<String> = nodes
        .iter()
        .map(|node| format!("{}/{}", node.hp, node.max_hp))
        .collect();
    parts.join(", ")
}

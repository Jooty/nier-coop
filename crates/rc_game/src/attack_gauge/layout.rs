use rc_proto::{AttackGaugeNode, MAX_ATTACK_GAUGE_NODES};

use crate::attack_gauge::{
    ATTACK_GAUGE_BOSS_HANDLE_OFFSET, ATTACK_GAUGE_DUMMY_HANDLE_OFFSET,
    ATTACK_GAUGE_FIRST_NODE_OFFSET, ATTACK_GAUGE_HP_FLOOR_PERCENT_RVA, ATTACK_GAUGE_MANAGER_RVA,
    ATTACK_GAUGE_MAP_POINTS_OFFSET, ATTACK_GAUGE_MODE_OFFSET, ATTACK_GAUGE_MODE_ONE_ACTOR,
    ATTACK_GAUGE_MODE_TWO_ACTORS, ATTACK_GAUGE_NODE_BONE_OFFSET, ATTACK_GAUGE_NODE_EXTRA_OFFSET,
    ATTACK_GAUGE_NODE_HP_OFFSET, ATTACK_GAUGE_NODE_KIND_OFFSET, ATTACK_GAUGE_NODE_MAX_HP_OFFSET,
    ATTACK_GAUGE_NODE_NEXT_OFFSET, ATTACK_GAUGE_NODE_OFFSET_OFFSET, ATTACK_GAUGE_NODE_SIZE_OFFSET,
    ATTACK_GAUGE_OPTIONS_OFFSET, ATTACK_GAUGE_PARTNER_HANDLE_OFFSET,
    ATTACK_GAUGE_REMAINING_SECONDS_OFFSET, ATTACK_GAUGE_TOTAL_SECONDS_OFFSET,
};
use crate::enemies::ActorHandle;
use crate::memory::GameMemory;

/// One node of the gauge as it sits in game memory: its address and its fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GaugeNodeReading {
    pub address: u64,
    pub node: AttackGaugeNode,
}

/// The game's attack gauge while it is open: the actors behind its handles, its clock, the map
/// points and option bytes it was opened with, and its nodes in list order. `more_nodes` is set
/// when the list holds more nodes than the wire carries.
#[derive(Clone, Debug, PartialEq)]
pub struct GaugeReading {
    pub mode: i32,
    pub boss: u64,
    pub partner: Option<u64>,
    pub dummy: u64,
    pub total_seconds: f32,
    pub remaining_seconds: f32,
    pub map_points: [i32; 3],
    pub options: [u8; 2],
    pub nodes: Vec<GaugeNodeReading>,
    pub more_nodes: bool,
}

pub fn manager_address<M: GameMemory>(memory: &M) -> u64 {
    memory.module_base() + u64::from(ATTACK_GAUGE_MANAGER_RVA)
}

/// The open gauge, or `None` when it is closed: the gauge counts as open while its mode is one
/// the opening functions write and both its boss handle and its dummy actor handle are live.
pub fn read_gauge<M: GameMemory>(memory: &M) -> Option<GaugeReading> {
    let manager = manager_address(memory);
    let mode = memory.read_i32(manager + ATTACK_GAUGE_MODE_OFFSET).ok()?;
    if mode != ATTACK_GAUGE_MODE_ONE_ACTOR && mode != ATTACK_GAUGE_MODE_TWO_ACTORS {
        return None;
    }
    let live = |offset: u64| {
        let handle = ActorHandle::read(memory, manager + offset)?;
        if handle.generation == 0 {
            return None;
        }
        handle.resolve(memory)
    };
    let boss = live(ATTACK_GAUGE_BOSS_HANDLE_OFFSET)?;
    let dummy = live(ATTACK_GAUGE_DUMMY_HANDLE_OFFSET)?;
    let partner = if mode == ATTACK_GAUGE_MODE_TWO_ACTORS {
        Some(live(ATTACK_GAUGE_PARTNER_HANDLE_OFFSET)?)
    } else {
        None
    };
    let int = |offset: u64| memory.read_i32(manager + offset).ok();
    let (nodes, more_nodes) = read_nodes(memory, manager)?;
    Some(GaugeReading {
        mode,
        boss,
        partner,
        dummy,
        total_seconds: memory
            .read_f32(manager + ATTACK_GAUGE_TOTAL_SECONDS_OFFSET)
            .ok()?,
        remaining_seconds: memory
            .read_f32(manager + ATTACK_GAUGE_REMAINING_SECONDS_OFFSET)
            .ok()?,
        map_points: [
            int(ATTACK_GAUGE_MAP_POINTS_OFFSET)?,
            int(ATTACK_GAUGE_MAP_POINTS_OFFSET + 4)?,
            int(ATTACK_GAUGE_MAP_POINTS_OFFSET + 8)?,
        ],
        options: [
            memory.read_u8(manager + ATTACK_GAUGE_OPTIONS_OFFSET).ok()?,
            memory
                .read_u8(manager + ATTACK_GAUGE_OPTIONS_OFFSET + 1)
                .ok()?,
        ],
        nodes,
        more_nodes,
    })
}

/// How many nodes the game's list holds right now, up to one more than the wire carries.
pub fn count_nodes<M: GameMemory>(memory: &M) -> Option<usize> {
    read_nodes(memory, manager_address(memory)).map(|(nodes, more)| nodes.len() + usize::from(more))
}

pub fn read_hp_floor_percent<M: GameMemory>(memory: &M) -> Option<i32> {
    memory
        .read_i32(memory.module_base() + u64::from(ATTACK_GAUGE_HP_FLOOR_PERCENT_RVA))
        .ok()
}

pub fn write_node_hp<M: GameMemory>(memory: &mut M, node: &GaugeNodeReading, hp: i32) -> bool {
    memory
        .write_i32(node.address + ATTACK_GAUGE_NODE_HP_OFFSET, hp)
        .is_ok()
}

fn read_nodes<M: GameMemory>(memory: &M, manager: u64) -> Option<(Vec<GaugeNodeReading>, bool)> {
    let mut nodes = Vec::new();
    let mut address = memory
        .read_u64(manager + ATTACK_GAUGE_FIRST_NODE_OFFSET)
        .ok()?;
    while address != 0 {
        if nodes.len() == MAX_ATTACK_GAUGE_NODES {
            return Some((nodes, true));
        }
        nodes.push(GaugeNodeReading {
            address,
            node: read_node(memory, address)?,
        });
        address = memory
            .read_u64(address + ATTACK_GAUGE_NODE_NEXT_OFFSET)
            .ok()?;
    }
    Some((nodes, false))
}

fn read_node<M: GameMemory>(memory: &M, address: u64) -> Option<AttackGaugeNode> {
    let int = |offset: u64| memory.read_i32(address + offset).ok();
    let float = |offset: u64| memory.read_f32(address + offset).ok();
    Some(AttackGaugeNode {
        bone: int(ATTACK_GAUGE_NODE_BONE_OFFSET)?,
        size: float(ATTACK_GAUGE_NODE_SIZE_OFFSET)?,
        offset: [
            float(ATTACK_GAUGE_NODE_OFFSET_OFFSET)?,
            float(ATTACK_GAUGE_NODE_OFFSET_OFFSET + 4)?,
            float(ATTACK_GAUGE_NODE_OFFSET_OFFSET + 8)?,
            float(ATTACK_GAUGE_NODE_OFFSET_OFFSET + 12)?,
        ],
        kind: int(ATTACK_GAUGE_NODE_KIND_OFFSET)?,
        extra: int(ATTACK_GAUGE_NODE_EXTRA_OFFSET)?,
        hp: int(ATTACK_GAUGE_NODE_HP_OFFSET)?,
        max_hp: int(ATTACK_GAUGE_NODE_MAX_HP_OFFSET)?,
    })
}

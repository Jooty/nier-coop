use std::sync::{Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::puppet::{MIRROR_SIDE_OFFSET_UNITS, PuppetPose, read_pose};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::MAX_PUPPETS;
use crate::puppet::actor::Puppet;
use crate::puppet::delay_setting::mirror_delay_ms;
use crate::puppet::error::PuppetError;
use crate::puppet::mirror::{MirrorDriver, Observed};

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());

#[derive(Debug, Clone)]
pub struct PuppetInfo {
    pub id: u32,
    pub actor: u64,
    pub spawned_in_phase: Option<String>,
    pub delay_ms: u16,
    pub liveness_is_trusted: bool,
    pub position: Option<[f32; 3]>,
    pub yaw: Option<f32>,
    pub motion_index: Option<i32>,
    pub motion_name: Option<String>,
    pub frame: Option<f32>,
    pub last_error: Option<String>,
}

#[derive(Debug)]
struct Entry {
    id: u32,
    puppet: Puppet,
    mirror: MirrorDriver,
    phase: Option<String>,
    observed: Observed,
    last_error: Option<String>,
}

#[derive(Debug)]
struct Registry {
    next_id: u32,
    entries: Vec<Entry>,
}

#[derive(Debug, Clone, Copy)]
enum Retire {
    OnRequest,
    PhaseChanged,
}

impl Registry {
    const fn new() -> Self {
        Self {
            next_id: 1,
            entries: Vec::new(),
        }
    }
}

pub fn spawn_mirror(build: &VerifiedBuild) -> Result<u32, PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if registry.entries.len() >= MAX_PUPPETS {
        return Err(PuppetError::TooManyPuppets);
    }
    let player = game.playable_actor()?;
    let start = read_pose(game.memory(), player)?.shifted_sideways(MIRROR_SIDE_OFFSET_UNITS);
    let id = registry.next_id;
    let puppet = Puppet::spawn(build, id, start)?;
    registry.next_id += 1;
    log_line(&format!(
        "puppet {id}: spawned actor {:#x}, liveness check trusted: {}",
        puppet.actor(),
        puppet.liveness_is_trusted()
    ));
    registry.entries.push(Entry {
        id,
        puppet,
        mirror: MirrorDriver::new(),
        phase: game.current_phase().ok(),
        observed: Observed::default(),
        last_error: None,
    });
    Ok(id)
}

pub fn despawn(build: &VerifiedBuild, id: Option<u32>) -> Result<usize, PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(id) = id
        && !registry.entries.iter().any(|entry| entry.id == id)
    {
        return Err(PuppetError::UnknownPuppet(id));
    }
    let mut removed = 0;
    let mut kept = Vec::new();
    for entry in registry.entries.drain(..) {
        if id.is_some_and(|wanted| wanted != entry.id) {
            kept.push(entry);
            continue;
        }
        removed += 1;
        retire(&game, entry, Retire::OnRequest);
    }
    registry.entries = kept;
    Ok(removed)
}

pub fn tick(build: &VerifiedBuild) {
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if registry.entries.is_empty() {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game.current_phase().ok();
    let mut kept = Vec::new();
    for mut entry in registry.entries.drain(..) {
        let phase_changed = phase.is_some() && entry.phase.is_some() && phase != entry.phase;
        if entry.puppet.liveness_is_trusted() && !entry.puppet.is_live(&game) {
            log_line(&format!(
                "puppet {}: actor is gone, forgetting it",
                entry.id
            ));
            continue;
        }
        if phase_changed {
            retire(&game, entry, Retire::PhaseChanged);
            continue;
        }
        match entry.mirror.tick(&game, &entry.puppet) {
            Ok(observed) => {
                entry.observed = observed;
                entry.last_error = None;
            }
            Err(error) => {
                let message = error.to_string();
                if entry.last_error.as_deref() != Some(message.as_str()) {
                    log_line(&format!("puppet {}: {message}", entry.id));
                }
                entry.last_error = Some(message);
            }
        }
        kept.push(entry);
    }
    registry.entries = kept;
}

pub fn snapshot() -> Vec<PuppetInfo> {
    let registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    registry
        .entries
        .iter()
        .map(|entry| {
            let pose: Option<PuppetPose> = entry.observed.pose;
            PuppetInfo {
                id: entry.id,
                actor: entry.puppet.actor(),
                spawned_in_phase: entry.phase.clone(),
                delay_ms: mirror_delay_ms(),
                liveness_is_trusted: entry.puppet.liveness_is_trusted(),
                position: pose.map(|pose| [pose.position.x, pose.position.y, pose.position.z]),
                yaw: pose.map(|pose| pose.yaw),
                motion_index: entry.observed.motion_index,
                motion_name: entry.observed.motion_name.clone(),
                frame: entry.observed.frame,
                last_error: entry.last_error.clone(),
            }
        })
        .collect()
}

fn retire(game: &Replicant<InProcessMemory>, entry: Entry, reason: Retire) {
    let id = entry.id;
    let can_destroy = match reason {
        Retire::OnRequest => !entry.puppet.liveness_is_trusted() || entry.puppet.is_live(game),
        Retire::PhaseChanged => entry.puppet.liveness_is_trusted() && entry.puppet.is_live(game),
    };
    if can_destroy {
        entry.puppet.despawn();
        log_line(&format!("puppet {id}: despawned ({reason:?})"));
    } else {
        log_line(&format!(
            "puppet {id}: dropped without destroy, the game removed it ({reason:?})"
        ));
    }
}

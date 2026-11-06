use std::sync::{Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::puppet::{LayerMotion, PuppetPose};
use rc_proto::{PartnerActivity, WeaponState};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::actor::Puppet;
use crate::puppet::error::PuppetError;
use crate::puppet::remote_driver::RemoteDriver;
use crate::puppet::weapon::PuppetWeapon;

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());

#[derive(Debug)]
struct Entry {
    id: u32,
    puppet: Puppet,
    driver: RemoteDriver,
    weapon: PuppetWeapon,
    phase: Option<String>,
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

pub fn spawn_ghost(build: &VerifiedBuild, start: PuppetPose) -> Result<u32, PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(alive) = registry.entries.first() {
        // One ghost per peer: adopt the puppet an earlier attempt left alive instead of a second one.
        return Ok(alive.id);
    }
    game.playable_actor()?;
    register(build, &game, &mut registry, start)
}

pub fn first_actor_address() -> Option<u64> {
    let registry = REGISTRY.try_lock().ok()?;
    registry.entries.first().map(|entry| entry.puppet.actor())
}

pub fn ghost_exists(id: u32) -> bool {
    let registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    registry.entries.iter().any(|entry| entry.id == id)
}

pub fn drive_ghost(
    build: &VerifiedBuild,
    id: u32,
    pose: PuppetPose,
    motion: Option<LayerMotion>,
    weapon: Option<WeaponState>,
    activity: PartnerActivity,
) -> Result<(), PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    let entry = registry
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or(PuppetError::UnknownPuppet(id))?;
    entry
        .driver
        .drive_remote(build, &game, &entry.puppet, pose, motion, activity)?;
    entry
        .weapon
        .drive(build, &game, &entry.puppet, entry.id, weapon);
    Ok(())
}

fn register(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    registry: &mut Registry,
    start: PuppetPose,
) -> Result<u32, PuppetError> {
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
        driver: RemoteDriver::new(),
        weapon: PuppetWeapon::default(),
        phase: game.current_phase().ok(),
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
    for entry in registry.entries.drain(..) {
        let phase_changed = phase.is_some() && entry.phase.is_some() && phase != entry.phase;
        if entry.puppet.liveness_is_trusted() && !entry.puppet.is_live(&game) {
            log_line(&format!(
                "puppet {}: actor is gone, forgetting it",
                entry.id
            ));
            let mut entry = entry;
            entry.weapon.despawn(&game, false);
            continue;
        }
        if phase_changed {
            retire(&game, entry, Retire::PhaseChanged);
            continue;
        }
        kept.push(entry);
    }
    registry.entries = kept;
}

fn retire(game: &Replicant<InProcessMemory>, mut entry: Entry, reason: Retire) {
    let id = entry.id;
    entry
        .weapon
        .despawn(game, matches!(reason, Retire::OnRequest));
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

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use rc_game::Replicant;
use rc_game::puppet::{
    BodyInfo, LayerMotion, PuppetPose, RetirePlan, SpawnResult, body_info, distance_between,
    read_actor_draw_report, retire_plan,
};
use rc_proto::{PartnerActivity, PartnerBody, WeaponState};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::puppet::actor::Puppet;
use crate::puppet::body_pack::release_if_ours;
use crate::puppet::book::PuppetBook;
use crate::puppet::error::PuppetError;
use crate::puppet::pack_guard::{ensure_pack_guard, guard, has_free_slot, still_guarded, unguard};
use crate::puppet::remote_driver::RemoteDriver;
use crate::puppet::weapon::PuppetWeapon;

static REGISTRY: Mutex<Registry> = Mutex::new(Registry::new());
const DRAW_REPORT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct Entry {
    id: u32,
    puppet: Puppet,
    driver: RemoteDriver,
    weapon: PuppetWeapon,
    book: PuppetBook,
    phase: Option<String>,
    body: &'static BodyInfo,
    pack: u64,
    pack_requested_by_us: bool,
    last_draw_report: Instant,
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

pub fn spawn_ghost(
    build: &VerifiedBuild,
    start: PuppetPose,
    body: PartnerBody,
    pack: (u64, bool),
) -> Result<SpawnResult, PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(alive) = registry.entries.first() {
        // One ghost per peer: adopt the puppet an earlier attempt left alive instead of a second one.
        return Ok(SpawnResult::AlreadyAlive {
            id: alive.id,
            body: alive.body.body,
        });
    }
    game.playable_actor()?;
    register(build, &game, &mut registry, start, body_info(body), pack).map(SpawnResult::Created)
}

pub fn first_actor_address() -> Option<u64> {
    let registry = REGISTRY.try_lock().ok()?;
    registry.entries.first().map(|entry| entry.puppet.actor())
}

pub fn book_actor_address() -> Option<u64> {
    let registry = REGISTRY.try_lock().ok()?;
    registry.entries.first()?.book.actor_address()
}

pub fn is_puppet_actor(actor: u64) -> bool {
    actor != 0
        && (first_actor_address() == Some(actor)
            || book_actor_address() == Some(actor)
            || crate::puppet::power_actor_addresses().contains(&actor))
}

pub fn book_actor_moved_to(build: &VerifiedBuild, position: [f32; 3]) -> Option<u64> {
    let registry = REGISTRY.try_lock().ok()?;
    let game = Replicant::new(InProcessMemory::new(build));
    registry
        .entries
        .first()?
        .book
        .move_to_world_position(&game, position)
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
    name_hash: u32,
) -> Result<(), PuppetError> {
    let game = Replicant::new(InProcessMemory::new(build));
    let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    let entry = registry
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or(PuppetError::UnknownPuppet(id))?;
    entry.driver.drive_remote(
        build,
        &game,
        &entry.puppet,
        pose,
        (motion, name_hash),
        activity,
    )?;
    if entry.body.weapon_shown {
        entry
            .weapon
            .drive(build, &game, &entry.puppet, entry.id, weapon);
    }
    entry.book.drive(
        build,
        &game,
        &entry.puppet,
        entry.id,
        (entry.body.body, net_clock_ms()),
    );
    Ok(())
}

fn register(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    registry: &mut Registry,
    start: PuppetPose,
    body: &'static BodyInfo,
    (pack, pack_requested_by_us): (u64, bool),
) -> Result<u32, PuppetError> {
    let id = registry.next_id;
    let guarded = ensure_pack_guard(build);
    if guarded && !has_free_slot() {
        return Err(PuppetError::NoFreePackGuardSlot);
    }
    let puppet = Puppet::spawn(build, id, start, body)?;
    if guarded {
        guard(puppet.actor(), pack);
    }
    registry.next_id += 1;
    log_line(&format!(
        "puppet {id}: spawned actor {:#x} with body {}, liveness check trusted: {}",
        puppet.actor(),
        body.pack_name,
        puppet.liveness_is_trusted()
    ));
    log_line(&format!("ghost body {}: spawned", body.pack_name));
    registry.entries.push(Entry {
        id,
        puppet,
        driver: RemoteDriver::new(body.idle_motions),
        weapon: PuppetWeapon::default(),
        book: PuppetBook::default(),
        phase: game.current_phase().ok(),
        body,
        pack,
        pack_requested_by_us,
        last_draw_report: Instant::now(),
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
        retire(build, &game, entry, Retire::OnRequest);
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
            entry.book.despawn(&game, false);
            unguard(entry.puppet.actor());
            release_body_pack(build, &entry);
            continue;
        }
        if phase_changed {
            retire(build, &game, entry, Retire::PhaseChanged);
            continue;
        }
        let mut entry = entry;
        report_draw_state(&game, &mut entry);
        kept.push(entry);
    }
    registry.entries = kept;
}

fn report_draw_state(game: &Replicant<InProcessMemory>, entry: &mut Entry) {
    if entry.last_draw_report.elapsed() < DRAW_REPORT_INTERVAL {
        return;
    }
    entry.last_draw_report = Instant::now();
    let puppet = read_actor_draw_report(game.memory(), entry.puppet.actor());
    let local = game
        .playable_actor()
        .ok()
        .and_then(|player| read_actor_draw_report(game.memory(), player).ok());
    let (Ok(puppet), Some(local)) = (puppet, local) else {
        log_line(&format!(
            "puppet {}: draw state unreadable (actor or local player)",
            entry.id
        ));
        return;
    };
    log_line(&format!(
        "puppet {} {:#x}: {}; you {}; {:.1} units apart",
        entry.id,
        entry.puppet.actor(),
        puppet.summary(),
        local.summary(),
        distance_between(puppet.position, local.position)
    ));
}

fn retire(
    build: &VerifiedBuild,
    game: &Replicant<InProcessMemory>,
    mut entry: Entry,
    reason: Retire,
) {
    let id = entry.id;
    let actor = entry.puppet.actor();
    entry
        .weapon
        .despawn(game, matches!(reason, Retire::OnRequest));
    entry
        .book
        .despawn(game, matches!(reason, Retire::OnRequest));
    let can_destroy = match reason {
        Retire::OnRequest => !entry.puppet.liveness_is_trusted() || entry.puppet.is_live(game),
        Retire::PhaseChanged => entry.puppet.liveness_is_trusted() && entry.puppet.is_live(game),
    };
    match retire_plan(can_destroy, still_guarded(actor)) {
        RetirePlan::ReleasePackWhichRemovesActor => {
            // The release detour removes the actor wearing the pack before the pack goes.
            if !release_body_pack(build, &entry) {
                unguard(actor);
                entry.puppet.despawn();
            }
            log_line(&format!("puppet {id}: despawned ({reason:?})"));
        }
        RetirePlan::DespawnThenReleasePack => {
            unguard(actor);
            let (body, pack, ours) = (entry.body, entry.pack, entry.pack_requested_by_us);
            entry.puppet.despawn();
            release_if_ours(build, body, pack, ours);
            log_line(&format!("puppet {id}: despawned ({reason:?})"));
        }
        RetirePlan::ForgetActorThenReleasePack => {
            unguard(actor);
            release_body_pack(build, &entry);
            log_line(&format!(
                "puppet {id}: dropped without destroy, the game removed it ({reason:?})"
            ));
        }
    }
}

fn release_body_pack(build: &VerifiedBuild, entry: &Entry) -> bool {
    release_if_ours(build, entry.body, entry.pack, entry.pack_requested_by_us)
}

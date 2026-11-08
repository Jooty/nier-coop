use core::ffi::c_void;
use core::ptr;
use std::collections::VecDeque;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::enemies::{creation_matrix_scale, spec_slot_index};
use rc_game::offsets::{
    ACTOR_SPAWN_SERIAL_OFFSET, CREATION_MATRIX_BYTES, PUT_ENEMY_ACTOR_FUNCTION_RVA,
};
use rc_proto::{ENEMY_SPEC_BYTES, MAX_ENEMY_ACTOR_NAME_BYTES};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

const MAX_REMEMBERED_CREATIONS: usize = 256;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_PUT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CREATIONS: LazyLock<Mutex<VecDeque<RememberedCreation>>> = LazyLock::new(Mutex::default);

type PutEnemyActor = unsafe extern "system" fn(
    *mut c_void,
    *const u8,
    i32,
    *const u8,
    *const u8,
    *const c_void,
    u8,
    f32,
    u8,
) -> *mut c_void;

#[derive(Debug, Clone)]
pub(super) struct RecordedCreation {
    pub(super) actor_name: String,
    pub(super) spec_slot: u16,
    pub(super) spawn_flag: u8,
    pub(super) attach_flag: u8,
    pub(super) scale: f32,
    pub(super) spec: Vec<u8>,
}

#[derive(Debug)]
struct RememberedCreation {
    actor: u64,
    serial: i32,
    creation: RecordedCreation,
}

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks MapManager::PutEnemyActor by its address")]
    // SAFETY: the RVA starts PutEnemyActor (map manager, name, group, matrix, spec, slot, flag,
    // delay, attach -> actor) in the verified build, read from its disassembly and from the
    // call in EnemyGroup::SpawnGroupAtPoint; the replacement has the same signature.
    unsafe {
        hook_address(
            "MapManager::PutEnemyActor",
            (build.module_base() + u64::from(PUT_ENEMY_ACTOR_FUNCTION_RVA)) as usize,
            put_enemy_actor_detour as *const () as *mut c_void,
            &ORIGINAL_PUT,
        );
    }
}

pub(super) fn take_creation(actor: u64, serial: i32) -> Option<RecordedCreation> {
    let mut creations = CREATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    let at = creations
        .iter()
        .position(|remembered| remembered.actor == actor && remembered.serial == serial)?;
    creations.remove(at).map(|remembered| remembered.creation)
}

#[expect(
    unsafe_code,
    reason = "calls the original PutEnemyActor through the trampoline MinHook built"
)]
extern "system" fn put_enemy_actor_detour(
    manager: *mut c_void,
    name: *const u8,
    group: i32,
    matrix: *const u8,
    spec: *const u8,
    slot: *const c_void,
    spawn_flag: u8,
    delay: f32,
    attach_flag: u8,
) -> *mut c_void {
    let original = ORIGINAL_PUT.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null_mut();
    }
    let captured = panic::catch_unwind(|| {
        capture_arguments(name, matrix, spec, slot, spawn_flag, attach_flag)
    })
    .ok()
    .flatten();
    // SAFETY: the trampoline continues into the original PutEnemyActor, which has this exact
    // signature; the arguments are the caller's, passed through unchanged.
    let actor = unsafe {
        core::mem::transmute::<*mut c_void, PutEnemyActor>(original)(
            manager,
            name,
            group,
            matrix,
            spec,
            slot,
            spawn_flag,
            delay,
            attach_flag,
        )
    };
    if let Some(creation) = captured {
        let _ = panic::catch_unwind(|| remember(actor as u64, creation));
    }
    actor
}

fn capture_arguments(
    name: *const u8,
    matrix: *const u8,
    spec: *const u8,
    slot: *const c_void,
    spawn_flag: u8,
    attach_flag: u8,
) -> Option<RecordedCreation> {
    let build = HOOKED_BUILD.get()?;
    let memory = InProcessMemory::new(build);
    let spec_slot = spec_slot_index(build.module_base(), slot as u64)?;
    let actor_name = read_actor_name(&memory, name as u64)?;
    let spec_bytes = memory
        .read_array::<ENEMY_SPEC_BYTES>(spec as u64)
        .ok()?
        .to_vec();
    let matrix_bytes = memory
        .read_array::<CREATION_MATRIX_BYTES>(matrix as u64)
        .ok()?;
    Some(RecordedCreation {
        actor_name,
        spec_slot,
        spawn_flag,
        attach_flag,
        scale: creation_matrix_scale(&matrix_bytes),
        spec: spec_bytes,
    })
}

fn read_actor_name(memory: &InProcessMemory, address: u64) -> Option<String> {
    let mut bytes = Vec::new();
    for offset in 0..=MAX_ENEMY_ACTOR_NAME_BYTES as u64 {
        let byte = memory.read_u8(address + offset).ok()?;
        if byte == 0 {
            return String::from_utf8(bytes)
                .ok()
                .filter(|name| !name.is_empty());
        }
        bytes.push(byte);
    }
    None
}

fn remember(actor: u64, creation: RecordedCreation) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    if actor == 0 {
        return;
    }
    let Ok(serial) = InProcessMemory::new(build).read_i32(actor + ACTOR_SPAWN_SERIAL_OFFSET) else {
        return;
    };
    let mut creations = CREATIONS.lock().unwrap_or_else(PoisonError::into_inner);
    if creations.len() >= MAX_REMEMBERED_CREATIONS {
        creations.pop_front();
    }
    creations.push_back(RememberedCreation {
        actor,
        serial,
        creation,
    });
}

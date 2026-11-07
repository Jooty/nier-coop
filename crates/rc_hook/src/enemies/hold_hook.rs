use core::ffi::c_void;
use core::ptr;
use std::collections::{HashMap, HashSet};
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_game::enemies::stop_own_step;
use rc_game::npcs::villager_runs_own_event;
use rc_game::offsets::{
    ACTOR_POST_UPDATE_VTABLE_OFFSET, ACTOR_PRE_UPDATE_VTABLE_OFFSET, ACTOR_TICK_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_TICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HELD_ENEMIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static GHOST_ENEMIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static STOPPED_DEAD_COPIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static OWN_STEPS_CUT: LazyLock<Mutex<HashMap<u64, f32>>> = LazyLock::new(Mutex::default);

type ActorTick = unsafe extern "system" fn(*mut c_void, f32) -> u8;
type PreUpdate = unsafe extern "system" fn(*mut c_void);
type PostUpdate = unsafe extern "system" fn(*mut c_void, f32) -> u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyTick {
    Free,
    Held,
    StoppedDead,
}

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks CActor::Tick by its address")]
    // SAFETY: the RVA starts the actor tick every non-player actor runs (actor, delta seconds ->
    // bool) in the verified build; the replacement has the same signature.
    unsafe {
        hook_address(
            "CActor::Tick",
            (build.module_base() + u64::from(ACTOR_TICK_FUNCTION_RVA)) as usize,
            actor_tick_detour as *const () as *mut c_void,
            &ORIGINAL_TICK,
        );
    }
}

pub(super) fn set_held(actors: HashSet<u64>) {
    *HELD_ENEMIES.lock().unwrap_or_else(PoisonError::into_inner) = actors;
}

pub(super) fn set_stopped_dead(actors: HashSet<u64>) {
    *STOPPED_DEAD_COPIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = actors;
}

pub(super) fn take_own_steps_cut() -> HashMap<u64, f32> {
    std::mem::take(&mut *OWN_STEPS_CUT.lock().unwrap_or_else(PoisonError::into_inner))
}

pub(super) fn set_ghosts(actors: HashSet<u64>) {
    *GHOST_ENEMIES.lock().unwrap_or_else(PoisonError::into_inner) = actors;
}

pub(super) fn is_ghost(actor: u64) -> bool {
    GHOST_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&actor)
}

pub(super) fn release_held(actor: u64) {
    HELD_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&actor);
}

fn copy_tick(actor: u64) -> CopyTick {
    if is_held(actor) {
        CopyTick::Held
    } else if STOPPED_DEAD_COPIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&actor)
    {
        CopyTick::StoppedDead
    } else {
        CopyTick::Free
    }
}

fn cut_own_step(actor: u64, may_fall: bool) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let Ok(Some(cut)) = stop_own_step(&mut InProcessMemory::new(build), actor, may_fall) else {
        return;
    };
    if may_fall {
        return;
    }
    let mut steps = OWN_STEPS_CUT.lock().unwrap_or_else(PoisonError::into_inner);
    let longest = steps.entry(actor).or_insert(0.0);
    *longest = longest.max(cut);
}

fn is_held(actor: u64) -> bool {
    let listed = HELD_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .contains(&actor);
    // A villager the enemy scan also lists must still run the talk or script this game started.
    listed
        && !HOOKED_BUILD
            .get()
            .is_some_and(|build| villager_runs_own_event(&InProcessMemory::new(build), actor))
}

#[expect(
    unsafe_code,
    reason = "calls the original actor tick through the trampoline, or a held actor's own vslots"
)]
extern "system" fn actor_tick_detour(actor: *mut c_void, delta_seconds: f32) -> u8 {
    let tick = panic::catch_unwind(|| copy_tick(actor as u64)).unwrap_or(CopyTick::Free);
    if tick == CopyTick::Held {
        // A held copy keeps vslots 0x4D0 and 0x4D8 (bookkeeping and its motion sequencer) but
        // not 0x4E0 and 0x4E8 (its AI and state steps); true keeps the actor alive.
        if let Some((pre, post)) = held_update_slots(actor as u64) {
            // SAFETY: both pointers come from the actor's vtable inside the game image and are
            // the slots the original tick calls first, with the same arguments, on this thread.
            unsafe {
                core::mem::transmute::<usize, PreUpdate>(pre)(actor);
                core::mem::transmute::<usize, PostUpdate>(post)(actor, delta_seconds);
            }
        }
        // The physics pass would move the copy by the step its motion or hit reaction queued.
        let _ = panic::catch_unwind(|| cut_own_step(actor as u64, false));
        return 1;
    }
    let original = ORIGINAL_TICK.load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    // SAFETY: the trampoline continues into the original tick, which has this exact signature.
    let alive =
        unsafe { core::mem::transmute::<*mut c_void, ActorTick>(original)(actor, delta_seconds) };
    if tick == CopyTick::StoppedDead {
        let _ = panic::catch_unwind(|| cut_own_step(actor as u64, true));
    }
    alive
}

fn held_update_slots(actor: u64) -> Option<(usize, usize)> {
    let build = HOOKED_BUILD.get()?;
    let memory = InProcessMemory::new(build);
    let image =
        build.module_base()..build.module_base() + u64::from(SUPPORTED_PE_IDENTITY.size_of_image);
    let vtable = memory.read_u64(actor).ok()?;
    let slot = |offset: u64| {
        memory
            .read_u64(vtable + offset)
            .ok()
            .filter(|function| image.contains(function))
            .map(|function| function as usize)
    };
    Some((
        slot(ACTOR_PRE_UPDATE_VTABLE_OFFSET)?,
        slot(ACTOR_POST_UPDATE_VTABLE_OFFSET)?,
    ))
}

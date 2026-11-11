use core::ffi::c_void;
use core::ptr;
use std::collections::{HashMap, HashSet};
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::GameMemory;
use replicant_game::build::SUPPORTED_PE_IDENTITY;
use replicant_game::enemies::{
    SlideReading, horizontal_distance, read_slide_reading, slide_check_text, stop_own_step,
    stop_own_velocity,
};
use replicant_game::npcs::{read_enemy_body_layer, villager_runs_own_event};
use replicant_game::offsets::{
    ACTOR_POST_UPDATE_VTABLE_OFFSET, ACTOR_PRE_UPDATE_VTABLE_OFFSET, ACTOR_TICK_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_TICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HELD_ENEMIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static GHOST_ENEMIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static STOPPED_DEAD_COPIES: LazyLock<Mutex<HashSet<u64>>> = LazyLock::new(Mutex::default);
static OWN_STEPS_CUT: LazyLock<Mutex<HashMap<u64, f32>>> = LazyLock::new(Mutex::default);
static SLIDE_LOGS: LazyLock<Mutex<HashMap<u64, SlideLog>>> = LazyLock::new(Mutex::default);

const SLIDE_CHECK_INTERVAL: Duration = Duration::from_secs(2);

type ActorTick = unsafe extern "system" fn(*mut c_void, f32) -> u8;
type PreUpdate = unsafe extern "system" fn(*mut c_void);
type PostUpdate = unsafe extern "system" fn(*mut c_void, f32) -> u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyTick {
    Free,
    Held,
    StoppedDead,
}

#[derive(Debug, Default)]
struct SlideLog {
    last_position: Option<[f32; 2]>,
    last_logged: Option<Instant>,
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

pub(super) fn hold_also(actors: impl IntoIterator<Item = u64>) {
    HELD_ENEMIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend(actors);
}

pub(super) fn set_stopped_dead(actors: HashSet<u64>) {
    let mut logs = SLIDE_LOGS.lock().unwrap_or_else(PoisonError::into_inner);
    let held = HELD_ENEMIES.lock().unwrap_or_else(PoisonError::into_inner);
    logs.retain(|actor, _| actors.contains(actor) || held.contains(actor));
    *STOPPED_DEAD_COPIES
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = actors;
}

pub(crate) fn cut_copy_step_before_move(actor: u64) {
    let may_fall = match copy_tick(actor) {
        CopyTick::Free => return,
        CopyTick::Held => false,
        CopyTick::StoppedDead => true,
    };
    cut_own_step(actor, may_fall);
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

fn cut_own_velocity(actor: u64, may_fall: bool) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let _ = stop_own_velocity(&mut InProcessMemory::new(build), actor, may_fall);
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
        let started = read_slide(actor as u64);
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
        let before_cut = read_slide(actor as u64);
        let _ = panic::catch_unwind(|| {
            cut_own_step(actor as u64, false);
            cut_own_velocity(actor as u64, false);
        });
        let _ = panic::catch_unwind(|| log_slide_check("held", actor as u64, started, before_cut));
        return 1;
    }
    let original = ORIGINAL_TICK.load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    let started = (tick == CopyTick::StoppedDead)
        .then(|| read_slide(actor as u64))
        .flatten();
    // SAFETY: the trampoline continues into the original tick, which has this exact signature.
    let alive =
        unsafe { core::mem::transmute::<*mut c_void, ActorTick>(original)(actor, delta_seconds) };
    if tick == CopyTick::StoppedDead {
        let before_cut = read_slide(actor as u64);
        let _ = panic::catch_unwind(|| {
            cut_own_step(actor as u64, true);
            cut_own_velocity(actor as u64, true);
        });
        let _ = panic::catch_unwind(|| log_slide_check("dead", actor as u64, started, before_cut));
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

fn read_slide(actor: u64) -> Option<SlideReading> {
    let build = HOOKED_BUILD.get()?;
    read_slide_reading(&InProcessMemory::new(build), actor).ok()
}

fn log_slide_check(
    kind: &str,
    actor: u64,
    started: Option<SlideReading>,
    before_cut: Option<SlideReading>,
) {
    let (Some(started), Some(before_cut), Some(after_cut), Some(build)) =
        (started, before_cut, read_slide(actor), HOOKED_BUILD.get())
    else {
        return;
    };
    let mut logs = SLIDE_LOGS.lock().unwrap_or_else(PoisonError::into_inner);
    let log = logs.entry(actor).or_default();
    let moved_since_last_tick = log
        .last_position
        .map(|last| horizontal_distance(last, started.ground_position));
    log.last_position = Some(started.ground_position);
    if log
        .last_logged
        .is_some_and(|at| at.elapsed() < SLIDE_CHECK_INTERVAL)
    {
        return;
    }
    log.last_logged = Some(Instant::now());
    drop(logs);
    let motion = read_enemy_body_layer(&InProcessMemory::new(build), actor)
        .ok()
        .flatten()
        .map(|layer| (layer.motion.index, layer.motion.frame));
    log_line(&slide_check_text(
        kind,
        actor,
        moved_since_last_tick,
        horizontal_distance(started.ground_position, after_cut.ground_position),
        &before_cut,
        &after_cut,
        motion,
    ));
}

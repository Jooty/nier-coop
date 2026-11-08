use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::enemies::{
    BOSS_BAR_DUMP_INTERVAL_MS, BOSS_BAR_INIT_FUNCTION_RVA, BOSS_BAR_SCRIPT_COMMAND_RVA,
    EnemySighting, IntervalClock, boss_bar_init_text, boss_bar_script_text,
    read_boss_bar_dump_text, read_boss_bar_handle, read_script_argument,
};
use rc_proto::EnemyId;

use crate::boss_messages::is_boss_fight_on;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::enemy_id_text;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_INIT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SCRIPT_COMMAND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SEEN_ENEMIES: LazyLock<Mutex<Vec<(u64, String)>>> = LazyLock::new(Mutex::default);
static DUMP_CLOCK: Mutex<IntervalClock> = Mutex::new(IntervalClock::new());
static DUMP_EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

type BarInit = unsafe extern "system" fn(*mut c_void, u32);
type ScriptCommand = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks two game functions by their addresses")]
    // SAFETY: both RVAs start the functions the replacements mirror in the verified build (bar
    // object, animation number) and (script command context); each detour calls the original.
    unsafe {
        hook_address(
            "boss bar init",
            (build.module_base() + u64::from(BOSS_BAR_INIT_FUNCTION_RVA)) as usize,
            bar_init_detour as *const () as *mut c_void,
            &ORIGINAL_INIT,
        );
        hook_address(
            "boss bar script command",
            (build.module_base() + u64::from(BOSS_BAR_SCRIPT_COMMAND_RVA)) as usize,
            script_command_detour as *const () as *mut c_void,
            &ORIGINAL_SCRIPT_COMMAND,
        );
    }
}

pub(super) fn log_boss_bar_state(build: &VerifiedBuild, seen: &[(EnemyId, EnemySighting)]) {
    *SEEN_ENEMIES.lock().unwrap_or_else(PoisonError::into_inner) = seen
        .iter()
        .map(|(id, sighting)| (sighting.actor, enemy_id_text(*id)))
        .collect();
    let mut clock = DUMP_CLOCK.lock().unwrap_or_else(PoisonError::into_inner);
    if !is_boss_fight_on() {
        clock.reset();
        return;
    }
    let now_ms = u64::try_from(DUMP_EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX);
    if clock.due(now_ms, BOSS_BAR_DUMP_INTERVAL_MS) {
        drop(clock);
        log_line(&read_boss_bar_dump_text(&InProcessMemory::new(build)));
    }
}

fn log_bar_init() {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let handle = read_boss_bar_handle(&memory);
    let enemy = handle
        .and_then(|handle| handle.resolve(&memory))
        .and_then(|actor| {
            SEEN_ENEMIES
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .find(|(seen, _)| *seen == actor)
                .map(|(_, text)| text.clone())
        });
    log_line(&boss_bar_init_text(handle, enemy.as_deref()));
}

fn log_script_command(command: u64) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    log_line(&boss_bar_script_text(
        read_script_argument(&memory, command, 0),
        read_script_argument(&memory, command, 1),
    ));
}

#[expect(
    unsafe_code,
    reason = "calls the original bar init through the trampoline"
)]
extern "system" fn bar_init_detour(object: *mut c_void, animation: u32) {
    let original = ORIGINAL_INIT.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, BarInit>(original)(object, animation) };
    let _ = panic::catch_unwind(log_bar_init);
}

#[expect(
    unsafe_code,
    reason = "calls the original script command through the trampoline"
)]
extern "system" fn script_command_detour(command: *mut c_void) {
    let _ = panic::catch_unwind(|| log_script_command(command as u64));
    let original = ORIGINAL_SCRIPT_COMMAND.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, ScriptCommand>(original)(command) };
}

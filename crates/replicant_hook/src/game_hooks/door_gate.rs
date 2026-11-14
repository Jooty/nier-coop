use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use replicant_game::Replicant;
use replicant_game::door_sync::DOOR_LOCK_MESSAGE_ID;
use replicant_game::offsets::{DOOR_MESSAGE_FUNCTION_RVA, DOOR_USE_MESSAGE_ID};
use replicant_game::zone_rules::{DoorKind, ZoneChangeSource};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::door_sync::{layout_id_of, lock_is_held_back, observe};
use crate::game_hooks::door_walk::note_door_use;
use crate::memory::InProcessMemory;
use crate::zone_rules::veto_zone_change;
use nt_hook::detour::hook_address;

const MAX_LOGGED_DOORS: usize = 200;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_DOOR_MESSAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LOGGED_DOORS: LazyLock<Mutex<HashSet<(String, i32)>>> = LazyLock::new(Mutex::default);

type DoorMessage = unsafe extern "system" fn(*mut c_void, u32, u64, i64, i32) -> u64;

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the door actor's message method by address"
    )]
    // SAFETY: the RVA starts CActorDoor's message method (vtable slot 0x5a0) in the verified build,
    // which the replacement mirrors.
    unsafe {
        hook_address(
            "CActorDoor_Message",
            (build.module_base() + u64::from(DOOR_MESSAGE_FUNCTION_RVA)) as usize,
            door_message_detour as *const () as *mut c_void,
            &ORIGINAL_DOOR_MESSAGE,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original door message method through the trampoline MinHook built"
)]
extern "system" fn door_message_detour(
    door: *mut c_void,
    message: u32,
    sender: u64,
    argument: i64,
    extra: i32,
) -> u64 {
    let original = ORIGINAL_DOOR_MESSAGE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    if message == DOOR_USE_MESSAGE_ID
        && panic::catch_unwind(|| door_use_refused(door)).unwrap_or(false)
    {
        // The door's own handler returns 0 for every message; nothing opens and no event is queued.
        return 0;
    }
    if message == DOOR_LOCK_MESSAGE_ID
        && panic::catch_unwind(|| fight_lock_held_back(door, argument)).unwrap_or(false)
    {
        return 0;
    }
    // SAFETY: the trampoline continues into the original method, which has this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, DoorMessage>(original)(
            door, message, sender, argument, extra,
        )
    };
    if let Some(build) = HOOKED_BUILD.get() {
        let _ = panic::catch_unwind(|| observe(build, door, message, argument));
    }
    result
}

fn fight_lock_held_back(door: *mut c_void, argument: i64) -> bool {
    HOOKED_BUILD
        .get()
        .is_some_and(|build| lock_is_held_back(build, door, argument))
}

fn door_use_refused(door: *mut c_void) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let phase = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .unwrap_or_default();
    let door_id = layout_id_of(build, door);
    let kind = DoorKind::of(&phase, door_id);
    note_door(&phase, door_id, kind);
    let DoorKind::LeavesTheArea { via } = kind else {
        return false;
    };
    let refused = veto_zone_change(
        &format!("exit door {door_id} ({via})"),
        ZoneChangeSource::ExitDoorInteraction,
    );
    if refused {
        log_line(&format!(
            "door gate: refused at the door: door {door_id} ({via})"
        ));
    } else if let Ok(position) = Replicant::new(InProcessMemory::new(build)).position() {
        note_door_use(position);
    }
    refused
}

fn note_door(phase: &str, door_id: i32, kind: DoorKind) {
    let mut logged = LOGGED_DOORS.lock().unwrap_or_else(PoisonError::into_inner);
    if logged.len() >= MAX_LOGGED_DOORS || !logged.insert((phase.to_owned(), door_id)) {
        return;
    }
    let verdict = match kind {
        DoorKind::LeavesTheArea { via } => format!("leaves the area (script runs {via})"),
        DoorKind::StaysInTheArea => "stays in this area".to_owned(),
    };
    log_line(&format!("doors: door {door_id} in {phase} {verdict}"));
}

use core::ffi::c_void;
use core::ptr;
use std::ffi::CStr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::GameMemory;
use replicant_game::offsets::{
    ACTOR_UNIQUE_ID_OFFSET, EVENT_ACTION_CALLBACK_REGISTER_FUNCTION_RVA,
    EVENT_ACTION_TALK_REGISTER_FUNCTION_RVA, TRIGGER_CALLBACK_NAME_CAPACITY,
};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::npcs::talk_press::{note_name, note_talk_id};

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_REGISTER_CALLBACK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_REGISTER_TALK_ID: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type Register = unsafe extern "system" fn(*mut c_void, i32, u64, u64, u64, u64) -> u64;

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the actor's event action registration functions by address"
    )]
    // SAFETY: each RVA starts a plain function (actor in rcx, mode in edx) in the verified build:
    // the callback name store (name in r8) and the numeric talk id store. The replacements take
    // the same six integer slots and pass them on unchanged before reading anything.
    unsafe {
        hook_address(
            "ActorEventActionCallback_Register",
            (base + u64::from(EVENT_ACTION_CALLBACK_REGISTER_FUNCTION_RVA)) as usize,
            register_callback_detour as *const () as *mut c_void,
            &ORIGINAL_REGISTER_CALLBACK,
        );
        hook_address(
            "ActorEventActionTalk_Register",
            (base + u64::from(EVENT_ACTION_TALK_REGISTER_FUNCTION_RVA)) as usize,
            register_talk_id_detour as *const () as *mut c_void,
            &ORIGINAL_REGISTER_TALK_ID,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original registration function through the trampoline MinHook built"
)]
extern "system" fn register_callback_detour(
    actor: *mut c_void,
    mode: i32,
    name: u64,
    third: u64,
    fourth: u64,
    fifth: u64,
) -> u64 {
    let original = ORIGINAL_REGISTER_CALLBACK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with these argument slots.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, Register>(original)(
            actor, mode, name, third, fourth, fifth,
        )
    };
    let _ = panic::catch_unwind(|| record_callback(actor as u64, mode, name));
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original registration function through the trampoline MinHook built"
)]
extern "system" fn register_talk_id_detour(
    actor: *mut c_void,
    mode: i32,
    talk_id: u64,
    third: u64,
    fourth: u64,
    fifth: u64,
) -> u64 {
    let original = ORIGINAL_REGISTER_TALK_ID.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with these argument slots.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, Register>(original)(
            actor, mode, talk_id, third, fourth, fifth,
        )
    };
    let _ = panic::catch_unwind(|| record_talk_id(actor as u64, mode, talk_id));
    result
}

fn record_callback(actor: u64, mode: i32, name_address: u64) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let Ok(actor_id) = memory.read_i32(actor + ACTOR_UNIQUE_ID_OFFSET) else {
        return;
    };
    let Some(name) = read_name(&memory, name_address) else {
        return;
    };
    note_name(build, actor_id, mode, &name);
}

fn record_talk_id(actor: u64, mode: i32, talk_id: u64) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let Ok(actor_id) = InProcessMemory::new(build).read_i32(actor + ACTOR_UNIQUE_ID_OFFSET) else {
        return;
    };
    note_talk_id(build, actor_id, mode, talk_id as u32 as i32);
}

fn read_name(memory: &InProcessMemory, address: u64) -> Option<String> {
    let mut bytes = Vec::with_capacity(TRIGGER_CALLBACK_NAME_CAPACITY);
    for offset in 0..TRIGGER_CALLBACK_NAME_CAPACITY as u64 {
        let byte = memory.read_u8(address + offset).ok()?;
        bytes.push(byte);
        if byte == 0 {
            return CStr::from_bytes_with_nul(&bytes)
                .ok()
                .map(|name| name.to_string_lossy().into_owned());
        }
    }
    None
}

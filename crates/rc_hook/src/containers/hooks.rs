use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::containers::{
    ACTION_RECORD_ID_OFFSET, ACTION_RECORD_KIND_OFFSET, ADD_ITEM_NUM_NATIVE_FIRST_RVA,
    ADD_ITEM_NUM_NATIVE_LAST_RVA, ADD_MONEY_FUNCTION_RVA, ADD_MONEY_NATIVE_FIRST_RVA,
    ADD_MONEY_NATIVE_LAST_RVA, DEFAULT_POINT_KIND_ARGUMENT, GAME_IMAGE_END_RVA,
    POINT_REGISTER_NATIVE_RVA, SCRIPT_POINT_RECORD_KIND, UNIQUE_EFFECT_CREATE_NATIVE_RVA,
    USE_ACTION_RUNNER_RVA,
};
use rc_game::offsets::ENTITY_RVA;
use rc_game::story_progress::{GAME_FLAG_SET_FUNCTION_RVA, SNOW_FLAG_SET_FUNCTION_RVA};
use rc_proto::StoryArray;
use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use crate::build_gate::VerifiedBuild;
use crate::containers::calls::lua_integer;
use crate::containers::state::{REGISTRY, WINDOW, now_ms};
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_USE_RUNNER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_POINT_REGISTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_EFFECT_CREATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_ADD_MONEY: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_FLAG_SET: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SNOW_FLAG_SET: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static FLAG_SETTER_DEPTH: AtomicU32 = AtomicU32::new(0);

type UseRunner = unsafe extern "system" fn(*mut c_void, u64, u64, u64) -> u64;
type ScriptNative = unsafe extern "system" fn(*mut c_void);
type AddMoney = unsafe extern "system" fn(*mut c_void, i32);
type FlagSetter = unsafe extern "system" fn(*mut c_void, u32);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    let hooks: [(&str, u32, *mut c_void, &AtomicPtr<c_void>); 6] = [
        (
            "EventAction::Execute",
            USE_ACTION_RUNNER_RVA,
            use_runner_detour as *const () as *mut c_void,
            &ORIGINAL_USE_RUNNER,
        ),
        (
            "_SetActionTargetToPointCallback",
            POINT_REGISTER_NATIVE_RVA,
            point_register_detour as *const () as *mut c_void,
            &ORIGINAL_POINT_REGISTER,
        ),
        (
            "_CreateUniqueEffect",
            UNIQUE_EFFECT_CREATE_NATIVE_RVA,
            effect_create_detour as *const () as *mut c_void,
            &ORIGINAL_EFFECT_CREATE,
        ),
        (
            "SaveBlock::AddMoney",
            ADD_MONEY_FUNCTION_RVA,
            add_money_detour as *const () as *mut c_void,
            &ORIGINAL_ADD_MONEY,
        ),
        (
            "FlagSet",
            GAME_FLAG_SET_FUNCTION_RVA,
            flag_set_detour as *const () as *mut c_void,
            &ORIGINAL_FLAG_SET,
        ),
        (
            "SnowFlagSet",
            SNOW_FLAG_SET_FUNCTION_RVA,
            snow_flag_set_detour as *const () as *mut c_void,
            &ORIGINAL_SNOW_FLAG_SET,
        ),
    ];
    for (label, rva, replacement, original) in hooks {
        #[expect(unsafe_code, reason = "hooks a game function by its address")]
        // SAFETY: each RVA starts the named function in the verified build and each replacement
        // has that function's signature.
        unsafe {
            hook_address(
                label,
                (base + u64::from(rva)) as usize,
                replacement,
                original,
            )
        };
    }
}

pub fn note_item_added(item_id: i32, added: i64, caller_rva: u32) {
    if !(ADD_ITEM_NUM_NATIVE_FIRST_RVA..ADD_ITEM_NUM_NATIVE_LAST_RVA).contains(&caller_rva) {
        return;
    }
    let (Ok(item_id), Ok(count)) = (u16::try_from(item_id), u8::try_from(added.clamp(0, 99)))
    else {
        return;
    };
    WINDOW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_item(item_id, count, now_ms());
}

#[expect(
    unsafe_code,
    reason = "calls the original use-action runner through the trampoline MinHook built"
)]
extern "system" fn use_runner_detour(record: *mut c_void, a: u64, b: u64, c: u64) -> u64 {
    let original = ORIGINAL_USE_RUNNER.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original runner, which has this exact signature.
    let result =
        unsafe { core::mem::transmute::<*mut c_void, UseRunner>(original)(record, a, b, c) };
    let _ = panic::catch_unwind(|| note_use(record));
    result
}

fn note_use(record: *mut c_void) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let (Ok(action_id), Ok(kind)) = (
        memory.read_i32(record as u64 + ACTION_RECORD_ID_OFFSET),
        memory.read_i32(record as u64 + ACTION_RECORD_KIND_OFFSET),
    ) else {
        return;
    };
    if kind != SCRIPT_POINT_RECORD_KIND {
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let (true, Some(local), Ok(action_id)) = (
        view.partner_in_play(),
        view.local.as_ref(),
        u16::try_from(action_id),
    ) else {
        return;
    };
    let finished = WINDOW.lock().unwrap_or_else(PoisonError::into_inner).begin(
        action_id,
        local.phase,
        now_ms(),
    );
    log_line(&format!(
        "containers: local use of action {action_id} in zone {}, collecting",
        local.phase
    ));
    if let Some(finished) = finished {
        super::tick::send_collected(net, finished);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original point registration native through the trampoline MinHook built"
)]
extern "system" fn point_register_detour(arguments: *mut c_void) {
    let original = ORIGINAL_POINT_REGISTER.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let _ = panic::catch_unwind(|| note_registration(arguments));
    // SAFETY: the trampoline continues into the original native, which takes the Lua argument block.
    unsafe { core::mem::transmute::<*mut c_void, ScriptNative>(original)(arguments) };
}

fn note_registration(arguments: *mut c_void) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let (Some(action_id), Some(point_id), Some(kind)) = (
        lua_integer(build, arguments, 1),
        lua_integer(build, arguments, 3),
        lua_integer(build, arguments, 9),
    ) else {
        return;
    };
    let kind = if kind == 0 {
        DEFAULT_POINT_KIND_ARGUMENT
    } else {
        kind
    };
    REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_registration(action_id, point_id, kind);
}

#[expect(
    unsafe_code,
    reason = "calls the original unique effect native through the trampoline MinHook built"
)]
extern "system" fn effect_create_detour(arguments: *mut c_void) {
    let original = ORIGINAL_EFFECT_CREATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let _ = panic::catch_unwind(|| note_effect(arguments));
    // SAFETY: the trampoline continues into the original native, which takes the Lua argument block.
    unsafe { core::mem::transmute::<*mut c_void, ScriptNative>(original)(arguments) };
}

fn note_effect(arguments: *mut c_void) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let (Some(handle), Some(effect_id), Some(point_id)) = (
        lua_integer(build, arguments, 0),
        lua_integer(build, arguments, 1),
        lua_integer(build, arguments, 2),
    ) else {
        return;
    };
    REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .note_effect(handle, effect_id, point_id);
}

#[expect(
    unsafe_code,
    reason = "calls the original AddMoney through the trampoline MinHook built"
)]
extern "system" fn add_money_detour(block: *mut c_void, delta: i32) {
    let original = ORIGINAL_ADD_MONEY.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let caller_rva = caller_rva_of_this_detour();
    // SAFETY: the trampoline continues into the original AddMoney, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, AddMoney>(original)(block, delta) };
    if delta > 0 && (ADD_MONEY_NATIVE_FIRST_RVA..ADD_MONEY_NATIVE_LAST_RVA).contains(&caller_rva) {
        WINDOW
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_gold(delta, now_ms());
    }
}

pub(super) fn add_money_for_partner(build: &VerifiedBuild, amount: i32) -> bool {
    let original = ORIGINAL_ADD_MONEY.load(Ordering::Acquire);
    if original.is_null() {
        return false;
    }
    let block = (build.module_base() + u64::from(ENTITY_RVA)) as *mut c_void;
    #[expect(
        unsafe_code,
        reason = "calls the original AddMoney through the trampoline MinHook built"
    )]
    // SAFETY: the trampoline continues into the original AddMoney (save block, delta); the save
    // block is the game's own global; this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, AddMoney>(original)(block, amount)
    };
    true
}

extern "system" fn flag_set_detour(owner: *mut c_void, id: u32) {
    flag_set_through(
        &ORIGINAL_FLAG_SET,
        StoryArray::GameFlags,
        owner,
        id,
        caller_rva_of_this_detour(),
    );
}

extern "system" fn snow_flag_set_detour(owner: *mut c_void, id: u32) {
    flag_set_through(
        &ORIGINAL_SNOW_FLAG_SET,
        StoryArray::SnowFlags,
        owner,
        id,
        caller_rva_of_this_detour(),
    );
}

#[expect(
    unsafe_code,
    reason = "calls an original flag setter through the trampoline MinHook built"
)]
fn flag_set_through(
    original: &AtomicPtr<c_void>,
    array: StoryArray,
    owner: *mut c_void,
    id: u32,
    caller_rva: u32,
) {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let outermost = FLAG_SETTER_DEPTH.fetch_add(1, Ordering::AcqRel) == 0;
    // SAFETY: the trampoline continues into the original setter, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, FlagSetter>(original)(owner, id) };
    FLAG_SETTER_DEPTH.fetch_sub(1, Ordering::AcqRel);
    if outermost
        && caller_rva != 0
        && let Ok(id) = u16::try_from(id)
    {
        WINDOW
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_flag(array, id, now_ms());
    }
}

#[inline(always)]
#[expect(
    unsafe_code,
    reason = "walks one stack frame to find who called the detour"
)]
fn caller_rva_of_this_detour() -> u32 {
    let Some(build) = HOOKED_BUILD.get() else {
        return 0;
    };
    let mut frames = [ptr::null_mut::<c_void>(); 1];
    // SAFETY: the buffer holds the one frame asked for; this helper is inlined into the detour, so
    // skipping one frame passes the detour and yields the return address in the game caller.
    let captured = unsafe { RtlCaptureStackBackTrace(1, 1, frames.as_mut_ptr(), ptr::null_mut()) };
    if captured == 0 {
        return 0;
    }
    let offset = (frames[0] as usize as u64).wrapping_sub(build.module_base());
    u32::try_from(offset)
        .ok()
        .filter(|rva| *rva < GAME_IMAGE_END_RVA)
        .unwrap_or(0)
}

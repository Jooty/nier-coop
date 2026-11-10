use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{REQUEST_PHASE_CHANGE_FUNCTION, SET_PENDING_SUB_PHASE_FUNCTION};
use automata_game::build::variant_of_image_size;
use automata_game::story_gate::{
    GateDecision, decide, refused_scene_request_line, refused_sub_phase_line,
};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const SCENE_NAME_READ_LIMIT: usize = 0x40;

static ORIGINAL_REQUEST_PHASE_CHANGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_PENDING_SUB_PHASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GATE_ARMED: AtomicBool = AtomicBool::new(false);
static GATE_CALLS: OnceLock<GameCalls> = OnceLock::new();
static REFUSED_SCENE_REQUESTS: Mutex<Option<HashSet<(i32, String)>>> = Mutex::new(None);
static REFUSED_SUB_PHASES: Mutex<Option<HashSet<u64>>> = Mutex::new(None);

thread_local! {
    static CALL_IS_OURS: Cell<bool> = const { Cell::new(false) };
}

type FourArguments = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

pub fn arm_story_gate() {
    if !GATE_ARMED.swap(true, Ordering::AcqRel) {
        log_line(
            "story: gate armed, scene requests and sub-phase changes that are not ours are refused",
        );
    }
}

pub fn run_as_ours<T>(call: impl FnOnce() -> T) -> T {
    let before = CALL_IS_OURS.with(|flag| flag.replace(true));
    let result = call();
    CALL_IS_OURS.with(|flag| flag.set(before));
    result
}

pub fn install_story_gate(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("story: image size matches no known exe, gate not installed");
        return;
    };
    if GATE_CALLS.set(GameCalls::new(build, variant)).is_err() {
        return;
    }
    let base = build.module_base() as usize;
    hook(
        "scene phase change request (story gate)",
        base + REQUEST_PHASE_CHANGE_FUNCTION.rva(variant) as usize,
        request_phase_change_detour as *const () as *mut c_void,
        &ORIGINAL_REQUEST_PHASE_CHANGE,
    );
    hook(
        "pending sub-phase setter (story gate)",
        base + SET_PENDING_SUB_PHASE_FUNCTION.rva(variant) as usize,
        set_pending_sub_phase_detour as *const () as *mut c_void,
        &ORIGINAL_SET_PENDING_SUB_PHASE,
    );
}

#[expect(
    unsafe_code,
    reason = "hooks a game function by address; both detours forward four register arguments"
)]
fn hook(label: &str, address: usize, replacement: *mut c_void, original: &AtomicPtr<c_void>) {
    // SAFETY: the address is a function of the verified build (a guess for the signature until
    // run); the replacement forwards four register arguments and returns the original's rax.
    unsafe {
        nc_hook::detour::hook_address(label, address, replacement, original);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original function through the trampoline MinHook built"
)]
fn call_original(slot: &AtomicPtr<c_void>, arguments: [usize; 4]) -> usize {
    let original = slot.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe {
        core::mem::transmute::<*mut c_void, FourArguments>(original)(
            arguments[0],
            arguments[1],
            arguments[2],
            arguments[3],
        )
    }
}

fn refuses_now() -> bool {
    let ours = CALL_IS_OURS.with(Cell::get);
    decide(GATE_ARMED.load(Ordering::Acquire), ours) == GateDecision::Refuse
}

extern "system" fn request_phase_change_detour(a: usize, b: usize, c: usize, d: usize) -> usize {
    if refuses_now() {
        log_refused_scene_request(b as u32 as i32, c);
        return 0;
    }
    call_original(&ORIGINAL_REQUEST_PHASE_CHANGE, [a, b, c, d])
}

extern "system" fn set_pending_sub_phase_detour(a: usize, b: usize, c: usize, d: usize) -> usize {
    if refuses_now() {
        log_refused_sub_phase(b as u64);
        return 0;
    }
    call_original(&ORIGINAL_SET_PENDING_SUB_PHASE, [a, b, c, d])
}

fn log_refused_scene_request(phase_id: i32, name_address: usize) {
    let name = GATE_CALLS
        .get()
        .map(|calls| calls.read_c_string_at(name_address as u64, SCENE_NAME_READ_LIMIT))
        .unwrap_or_default();
    let Ok(mut guard) = REFUSED_SCENE_REQUESTS.lock() else {
        return;
    };
    if guard
        .get_or_insert_with(HashSet::new)
        .insert((phase_id, name.clone()))
    {
        log_line(&refused_scene_request_line(phase_id, &name));
    }
}

fn log_refused_sub_phase(second_argument: u64) {
    let Ok(mut guard) = REFUSED_SUB_PHASES.lock() else {
        return;
    };
    if guard
        .get_or_insert_with(HashSet::new)
        .insert(second_argument)
    {
        log_line(&refused_sub_phase_line(second_argument));
    }
}

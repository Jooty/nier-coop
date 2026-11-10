use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{
    CURRENT_PHASE_ID_VALUE, CURRENT_SUB_PHASE_NAME_VALUE, REQUEST_PHASE_CHANGE_FUNCTION,
    SET_PENDING_SUB_PHASE_FUNCTION,
};
use automata_game::build::variant_of_image_size;
use automata_game::start::{SUB_PHASE_NAME_BUFFER_LEN, SubPhaseReading};
use automata_game::story_gate::{
    CurrentPhase, SceneRequest, SceneRequestDecision, StoryRule, SubPhaseDecision,
    decide_own_travel_request, decide_scene_request, decide_sub_phase, foreign_scene_request_line,
    foreign_sub_phase_line, is_own_travel_request, own_travel_scene_request_line,
    refused_scene_request_line, refused_sub_phase_line, story_status_line,
};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const SCENE_NAME_READ_LIMIT: usize = 0x40;
const NO_PHASE_ID: i32 = -1;

static ORIGINAL_REQUEST_PHASE_CHANGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_PENDING_SUB_PHASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GATE_ARMED: AtomicBool = AtomicBool::new(false);
static GATE_HOOKED: AtomicBool = AtomicBool::new(false);
static TRAVEL_SCENE_REQUEST_EXPECTED: AtomicBool = AtomicBool::new(false);
static GATE: OnceLock<(GameCalls, StoryRule)> = OnceLock::new();
static LOGGED_REQUESTS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

thread_local! {
    static CALL_IS_OURS: Cell<bool> = const { Cell::new(false) };
}

type FourArguments = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

pub fn arm_story_gate() {
    if !GATE_HOOKED.load(Ordering::Acquire) || GATE_ARMED.swap(true, Ordering::AcqRel) {
        return;
    }
    log_line(match story_rule() {
        StoryRule::Hold => {
            "story: gate armed, scene requests and sub-phase changes that are not ours are refused (the title and the current phase pass)"
        }
        StoryRule::Allow => {
            "story: gate armed with story = allow, scene requests and sub-phase changes that are not ours are only logged"
        }
    });
}

pub fn disarm_story_gate(reason: &str) {
    if GATE_ARMED.swap(false, Ordering::AcqRel) {
        log_line(&format!("story: gate disarmed ({reason})"));
    }
}

pub fn story_gate_status() -> String {
    story_status_line(story_rule(), GATE_ARMED.load(Ordering::Acquire))
}

/// While set, the game's next scene requests count as ours: the game's transport asks for the
/// scene change by itself, frames after our call.
pub fn expect_travel_scene_request(expected: bool) {
    TRAVEL_SCENE_REQUEST_EXPECTED.store(expected, Ordering::Release);
}

pub fn run_as_ours<T>(call: impl FnOnce() -> T) -> T {
    let before = CALL_IS_OURS.with(|flag| flag.replace(true));
    let result = call();
    CALL_IS_OURS.with(|flag| flag.set(before));
    result
}

pub fn install_story_gate(build: VerifiedBuild, rule: StoryRule) -> bool {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("story: image size matches no known exe, gate not installed");
        return false;
    };
    if GATE.set((GameCalls::new(build, variant), rule)).is_err() {
        return GATE_HOOKED.load(Ordering::Acquire);
    }
    let base = build.module_base() as usize;
    let scene_hooked = hook(
        "scene phase change request (story gate)",
        base + REQUEST_PHASE_CHANGE_FUNCTION.rva(variant) as usize,
        request_phase_change_detour as *const () as *mut c_void,
        &ORIGINAL_REQUEST_PHASE_CHANGE,
    );
    let sub_phase_hooked = hook(
        "pending sub-phase setter (story gate)",
        base + SET_PENDING_SUB_PHASE_FUNCTION.rva(variant) as usize,
        set_pending_sub_phase_detour as *const () as *mut c_void,
        &ORIGINAL_SET_PENDING_SUB_PHASE,
    );
    let hooked = scene_hooked && sub_phase_hooked;
    if !hooked {
        log_line("story: a gate hook is missing, the gate stays off");
    }
    GATE_HOOKED.store(hooked, Ordering::Release);
    hooked
}

fn story_rule() -> StoryRule {
    GATE.get().map_or(StoryRule::default(), |(_, rule)| *rule)
}

#[expect(
    unsafe_code,
    reason = "hooks a game function by address; both detours forward four register arguments"
)]
fn hook(
    label: &str,
    address: usize,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) -> bool {
    // SAFETY: the address is a function of the verified build (a guess for the signature until
    // run); the replacement forwards four register arguments and returns the original's rax.
    unsafe { nc_hook::detour::hook_address(label, address, replacement, original) }
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

fn read_name(calls: &GameCalls, name_address: usize) -> Option<String> {
    (name_address != 0).then(|| calls.read_c_string_at(name_address as u64, SCENE_NAME_READ_LIMIT))
}

fn read_current_phase(calls: &GameCalls) -> (i32, Option<String>) {
    let id = calls
        .read_i32(CURRENT_PHASE_ID_VALUE)
        .unwrap_or(NO_PHASE_ID);
    let name = calls
        .read_bytes(CURRENT_SUB_PHASE_NAME_VALUE, SUB_PHASE_NAME_BUFFER_LEN)
        .ok()
        .and_then(|bytes| match SubPhaseReading::of_bytes(&bytes) {
            SubPhaseReading::Name(name) => Some(name),
            SubPhaseReading::NotAName => None,
        });
    (id, name)
}

extern "system" fn request_phase_change_detour(a: usize, b: usize, c: usize, d: usize) -> usize {
    let armed = GATE_ARMED.load(Ordering::Acquire);
    let by_our_call = CALL_IS_OURS.with(Cell::get);
    let by_our_travel = !by_our_call
        && GATE.get().is_some_and(|(calls, _)| {
            is_own_travel_request(
                TRAVEL_SCENE_REQUEST_EXPECTED.load(Ordering::Acquire),
                calls.fast_travel_step(),
            )
        });
    if by_our_travel && let Some((calls, _)) = GATE.get() {
        let phase_id = b as u32 as i32;
        let name = read_name(calls, c).unwrap_or_default();
        log_line(&own_travel_scene_request_line(phase_id, &name));
        if armed
            && decide_own_travel_request(phase_id) == SceneRequestDecision::PassToTitleAndDisarm
        {
            disarm_story_gate("our own jump leads back to the title");
        }
    }
    let ours = by_our_call || by_our_travel;
    if let (true, false, Some((calls, rule))) = (armed, ours, GATE.get()) {
        let phase_id = b as u32 as i32;
        let name = read_name(calls, c);
        let (current_id, current_name) = read_current_phase(calls);
        let decision = decide_scene_request(
            armed,
            *rule,
            SceneRequest {
                ours,
                phase_id,
                phase_name: name.as_deref(),
            },
            CurrentPhase {
                id: current_id,
                name: current_name.as_deref(),
            },
        );
        let name = name.unwrap_or_default();
        match decision {
            SceneRequestDecision::Pass => {}
            SceneRequestDecision::PassToTitleAndDisarm => {
                disarm_story_gate("the game goes back to the title");
            }
            SceneRequestDecision::PassAndLog => {
                log_once(foreign_scene_request_line(phase_id, &name));
            }
            SceneRequestDecision::Refuse => {
                log_once(refused_scene_request_line(phase_id, &name));
                return 0;
            }
        }
    }
    call_original(&ORIGINAL_REQUEST_PHASE_CHANGE, [a, b, c, d])
}

extern "system" fn set_pending_sub_phase_detour(a: usize, b: usize, c: usize, d: usize) -> usize {
    let armed = GATE_ARMED.load(Ordering::Acquire);
    let ours = CALL_IS_OURS.with(Cell::get);
    if let (true, false, Some((calls, rule))) = (armed, ours, GATE.get()) {
        let phase_id = a as u32 as i32;
        let name = read_name(calls, b);
        let (current_id, current_name) = read_current_phase(calls);
        let decision = decide_sub_phase(
            armed,
            *rule,
            SceneRequest {
                ours,
                phase_id,
                phase_name: name.as_deref(),
            },
            CurrentPhase {
                id: current_id,
                name: current_name.as_deref(),
            },
        );
        let name = name.unwrap_or_default();
        match decision {
            SubPhaseDecision::Pass => {}
            SubPhaseDecision::PassAndLog => log_once(foreign_sub_phase_line(phase_id, &name)),
            SubPhaseDecision::Refuse => {
                log_once(refused_sub_phase_line(phase_id, &name));
                return 0;
            }
        }
    }
    call_original(&ORIGINAL_SET_PENDING_SUB_PHASE, [a, b, c, d])
}

fn log_once(line: String) {
    let Ok(mut guard) = LOGGED_REQUESTS.lock() else {
        return;
    };
    if guard.get_or_insert_with(HashSet::new).insert(line.clone()) {
        log_line(&line);
    }
}

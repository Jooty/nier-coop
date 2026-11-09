use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::thread;
use std::time::Duration;

use rc_game::marker::read_world_flags;
use rc_game::offsets::{START_TALK_NATIVE_RVA, START_TALK_UNIQUE_ACTOR_NATIVE_RVA};
use rc_game::quiet_end::{
    LUA_ARG_PAYLOAD_OFFSET, LUA_ARG_TAG_OFFSET, NATIVE_STATE_OFFSET, QUIET_END_SKIPPED_NATIVES,
    lua_arg_as_int, lua_arg_offset, talk_start_verdict,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;
use crate::game_hooks::script_runner_log::{last_pop, note_native};
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::pickups::injected_end_now;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_START_TALK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_TALK_UNIQUE_ACTOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

const TALK_WINDOW_CHECK_DELAY: Duration = Duration::from_millis(500);
const TALK_START_ARG_COUNT: u64 = 3;

type LuaNative = unsafe extern "system" fn(*mut c_void) -> i32;

pub(super) fn install(build: VerifiedBuild) {
    let _ = HOOKED_BUILD.set(build);
    let base = build.module_base();
    #[expect(unsafe_code, reason = "hooks the talk start natives by address")]
    // SAFETY: each RVA starts a talk start Lua native (lua args -> result count) in the verified
    // build, and each replacement has the same signature.
    unsafe {
        hook_address(
            "_StartTalk native",
            (base + u64::from(START_TALK_NATIVE_RVA)) as usize,
            start_talk_detour as *const () as *mut c_void,
            &ORIGINAL_START_TALK,
        );
        hook_address(
            "_StartTalkUniqueActorNotChangeEndTalkMotion native",
            (base + u64::from(START_TALK_UNIQUE_ACTOR_NATIVE_RVA)) as usize,
            start_talk_unique_actor_detour as *const () as *mut c_void,
            &ORIGINAL_START_TALK_UNIQUE_ACTOR,
        );
    }
}

extern "system" fn start_talk_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(&ORIGINAL_START_TALK, START_TALK_NATIVE_RVA, args)
}

extern "system" fn start_talk_unique_actor_detour(args: *mut c_void) -> i32 {
    run_unless_quiet(
        &ORIGINAL_START_TALK_UNIQUE_ACTOR,
        START_TALK_UNIQUE_ACTOR_NATIVE_RVA,
        args,
    )
}

#[expect(unsafe_code, reason = "calls the original talk start native")]
fn run_unless_quiet(original: &AtomicPtr<c_void>, native_rva: u32, args: *mut c_void) -> i32 {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let native = if native_rva == START_TALK_NATIVE_RVA {
        "_StartTalk"
    } else {
        "_StartTalkUniqueActorNotChangeEndTalkMotion"
    };
    let _ = panic::catch_unwind(|| note_native(native));
    let first_poll = panic::catch_unwind(|| is_first_poll(args)).unwrap_or(false);
    if first_poll {
        let skip = panic::catch_unwind(|| decide_and_log(native_rva, args)).unwrap_or(false);
        if skip {
            return 0;
        }
    }
    // SAFETY: the trampoline continues into the original native with this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaNative>(original)(args) }
}

#[expect(
    unsafe_code,
    reason = "reads the native's state word from its argument block"
)]
fn is_first_poll(args: *mut c_void) -> bool {
    // SAFETY: the game passes a live native block whose state word sits at this offset.
    unsafe {
        *args
            .cast::<u8>()
            .add(NATIVE_STATE_OFFSET as usize)
            .cast::<i32>()
            == 0
    }
}

#[expect(
    unsafe_code,
    reason = "reads the native's Lua arguments from its argument block"
)]
fn read_talk_args(args: *mut c_void) -> Vec<i32> {
    (0..TALK_START_ARG_COUNT)
        .map(|index| {
            let arg = lua_arg_offset(index) as usize;
            // SAFETY: the block holds at least TALK_START_ARG_COUNT argument slots, as both natives read.
            unsafe {
                let tag = *args
                    .cast::<u8>()
                    .add(arg + LUA_ARG_TAG_OFFSET as usize)
                    .cast::<i32>();
                let payload = *args
                    .cast::<u8>()
                    .add(arg + LUA_ARG_PAYLOAD_OFFSET as usize)
                    .cast::<[u8; 8]>();
                lua_arg_as_int(tag, payload)
            }
        })
        .collect()
}

fn decide_and_log(native_rva: u32, args: *mut c_void) -> bool {
    let name = QUIET_END_SKIPPED_NATIVES
        .iter()
        .find(|native| native.rva == native_rva)
        .map_or("?", |native| native.name);
    let injected = injected_end_now();
    let verdict = talk_start_verdict(
        native_rva,
        injected.is_some(),
        injected.as_ref().is_some_and(|end| end.shared_event),
    );
    let now = net_clock_ms();
    let script_call = last_pop().map_or_else(
        || "none".to_owned(),
        |(call, at)| format!("{call} ({} ms ago)", now.saturating_sub(at)),
    );
    let window = injected.as_ref().map_or_else(
        || "closed".to_owned(),
        |end| format!("open for {}", end.callback),
    );
    let mask = HOOKED_BUILD
        .get()
        .and_then(|build| read_world_flags(&InProcessMemory::new(build)).ok())
        .map_or_else(
            || "?".to_owned(),
            |flags| format!("{:#x}", flags.control_mask),
        );
    log_line(&format!(
        "talk start: {name} args {:?}, script call {script_call}, injected End window {window}, pause mask {mask}, {}",
        read_talk_args(args),
        verdict.describe()
    ));
    if verdict.skips() {
        return true;
    }
    thread::spawn(|| {
        thread::sleep(TALK_WINDOW_CHECK_DELAY);
        let _ = panic::catch_unwind(log_talk_state_after_start);
    });
    false
}

fn log_talk_state_after_start() {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let Ok(flags) = read_world_flags(&memory) else {
        return;
    };
    log_line(&format!(
        "talk start: 500 ms later talk window open {}, event scene {}, pause mask {:#x}",
        flags.talk_window_open, flags.event_scene, flags.control_mask
    ));
}

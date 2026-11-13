use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::thread;
use std::time::Duration;

use replicant_game::dialogue::{HeldTalkStart, HeldTalkStep};
use replicant_game::marker::read_world_flags;
use replicant_game::offsets::{START_TALK_NATIVE_RVA, START_TALK_UNIQUE_ACTOR_NATIVE_RVA};
use replicant_game::quiet_end::{
    LUA_ARG_PAYLOAD_OFFSET, LUA_ARG_TAG_OFFSET, LUA_NATIVE_DONE, LUA_NATIVE_STILL_RUNNING,
    NATIVE_STATE_OFFSET, QUIET_END_SKIPPED_NATIVES, lua_arg_as_int, lua_arg_offset,
    talk_start_verdict,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::script_runner_log::{last_pop, note_native};
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::pickups::injected_end_now;
use nc_hook::detour::hook_address;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_START_TALK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_TALK_UNIQUE_ACTOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HELD_TALK_START: Mutex<HeldTalkStart> = Mutex::new(HeldTalkStart::new());

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
        if let Ok(Some(result)) = panic::catch_unwind(|| poll_held_talk_start(args)) {
            return result;
        }
        let skip = panic::catch_unwind(|| decide_and_log(native_rva, args)).unwrap_or(false);
        if skip {
            return panic::catch_unwind(|| hold_skipped_talk_start(args))
                .unwrap_or(LUA_NATIVE_DONE);
        }
    }
    // SAFETY: the trampoline continues into the original native with this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaNative>(original)(args) }
}

fn hold_skipped_talk_start(args: *mut c_void) -> i32 {
    let Some(talk_id) = read_talk_args(args)
        .first()
        .and_then(|talk_id| u32::try_from(*talk_id).ok())
    else {
        return LUA_NATIVE_DONE;
    };
    HELD_TALK_START
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .begin(args as u64, talk_id, net_clock_ms());
    log_line(&format!(
        "talk start: your copy of talk {talk_id} waits in its start while the partner's talk {talk_id} runs (shown here by the mirror), so its scene goes on at the partner's pace"
    ));
    poll_held_talk_start(args).unwrap_or(LUA_NATIVE_DONE)
}

fn poll_held_talk_start(args: *mut c_void) -> Option<i32> {
    let mut held = HELD_TALK_START
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let talk_id = held.holds(args as u64)?;
    let same_talk = read_talk_args(args)
        .first()
        .is_some_and(|asked| u32::try_from(*asked).is_ok_and(|asked| asked == talk_id));
    if !same_talk {
        // Another start in the same native block: the held one is gone with its script.
        held.forget();
        return None;
    }
    let (partner_on_talk, partner_just_ended_it) = crate::dialogue::partner_talk_state(talk_id);
    match held.poll(partner_on_talk, partner_just_ended_it, net_clock_ms()) {
        HeldTalkStep::Wait => Some(LUA_NATIVE_STILL_RUNNING),
        HeldTalkStep::Release(release) => {
            drop(held);
            log_line(&format!(
                "talk start: your copy of talk {talk_id} goes on, {}",
                release.describe()
            ));
            Some(LUA_NATIVE_DONE)
        }
    }
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
    if let Some((call, _)) = last_pop() {
        crate::dialogue::note_script_talk_start(&call);
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

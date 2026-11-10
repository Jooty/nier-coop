use core::ffi::{CStr, c_char, c_void};
use core::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock, PoisonError};

use replicant_game::stay_together::FollowRole;
use replicant_game::story_progress::{StoryArrays, StoryKey};
use replicant_game::story_writes::{
    CALL_FUNCTION_RVA, EndWatch, RunningCall, ScriptCallTracker, TALK_TAG_RUN_CALLBACK_RVA,
    rebaseline_over_mirror_fixes, talk_tag_function_name, talker_write_batches,
};
use replicant_proto::{Message, StoryChange, StoryWrite};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::net::{self, NetState, net_clock_ms};
use crate::story_progress::{describe_changes, read_live_arrays};

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_CALL_FUNCTION: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_RUN_CALLBACK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TRACKER: Mutex<ScriptCallTracker> = Mutex::new(ScriptCallTracker::new());
static STORY_BEFORE_TALK_CALLBACK: Mutex<Option<StoryArrays>> = Mutex::new(None);
static END_WATCH: Mutex<EndWatch> = Mutex::new(EndWatch::new());
static STORY_BEFORE_END: Mutex<Option<StoryArrays>> = Mutex::new(None);
static HOLDS_TO_START: Mutex<Vec<StoryKey>> = Mutex::new(Vec::new());

type CallFunction = unsafe extern "system" fn(*mut c_void, *const c_char, *mut c_void) -> u8;
type RunCallback = unsafe extern "system" fn(*const c_char, i32) -> u8;

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the script call start and the talk tag callback queue by address"
    )]
    // SAFETY: each RVA starts the function the replacement mirrors in the verified build
    // (ScriptManager::CallFunction, TalkTag::RunCallback; see docs off-hand/story-choices.md).
    unsafe {
        hook_address(
            "ScriptManager::CallFunction",
            (base + u64::from(CALL_FUNCTION_RVA)) as usize,
            call_function_detour as *const () as *mut c_void,
            &ORIGINAL_CALL_FUNCTION,
        );
        hook_address(
            "TalkTag::RunCallback",
            (base + u64::from(TALK_TAG_RUN_CALLBACK_RVA)) as usize,
            run_callback_detour as *const () as *mut c_void,
            &ORIGINAL_RUN_CALLBACK,
        );
    }
}

pub fn running_call() -> Option<RunningCall> {
    TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .running()
}

pub fn queued_by_talk_tag(function_name: &[u8]) -> bool {
    TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .queued_by_talk_tag(function_name, net_clock_ms())
}

pub fn take_holds() -> Vec<StoryKey> {
    std::mem::take(
        &mut *HOLDS_TO_START
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    )
}

pub fn note_mirror_fixes_applied(fixes: &[StoryChange]) {
    if let Some(baseline) = STORY_BEFORE_END
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        rebaseline_over_mirror_fixes(baseline, fixes);
    }
    if let Some(baseline) = STORY_BEFORE_TALK_CALLBACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_mut()
    {
        rebaseline_over_mirror_fixes(baseline, fixes);
    }
}

pub fn begin_end_watch() {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    *STORY_BEFORE_END
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = read_live_arrays(build);
    END_WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .begin(net_clock_ms());
}

pub(super) fn clear_end_watch() {
    END_WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clear();
    *STORY_BEFORE_END
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
}

pub(super) fn send_writes_after_end(build: &VerifiedBuild) {
    let settled = END_WATCH
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_if_settled(net_clock_ms());
    if !settled {
        return;
    }
    let Some(earlier) = STORY_BEFORE_END
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
    else {
        return;
    };
    let Some(now) = read_live_arrays(build) else {
        return;
    };
    let batches = talker_write_batches(&earlier, &now);
    let total: usize = batches.iter().map(Vec::len).sum();
    log_line(&format!(
        "talk-write: after the end, forwarded {total} changes"
    ));
    let Some(net) = net::handle() else {
        return;
    };
    for changes in batches {
        log_line(&format!(
            "talk-write: after the end, sending {} changes to the host: {}",
            changes.len(),
            describe_changes(&changes)
        ));
        HOLDS_TO_START
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(changes.iter().copied().map(StoryKey::of));
        net.send_relayed(Message::StoryWrite(StoryWrite { changes }));
    }
}

pub(super) fn talk_callback_running() -> bool {
    TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .talk_callback_running(net_clock_ms())
}

pub(super) fn send_talker_writes(build: &VerifiedBuild, keep_watching: bool) {
    let mut before = STORY_BEFORE_TALK_CALLBACK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(earlier) = before.as_ref() else {
        return;
    };
    let Some(now) = read_live_arrays(build) else {
        return;
    };
    let batches = talker_write_batches(earlier, &now);
    *before = keep_watching.then_some(now);
    drop(before);
    let Some(net) = net::handle() else {
        return;
    };
    let callback = running_call().map_or(0, |call| call.name_hash);
    for changes in batches {
        log_line(&format!(
            "talk-write: callback={callback:016x} forwarded {} changes to the host: {}",
            changes.len(),
            describe_changes(&changes)
        ));
        HOLDS_TO_START
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(changes.iter().copied().map(StoryKey::of));
        net.send_relayed(Message::StoryWrite(StoryWrite { changes }));
    }
}

fn local_is_connected_guest() -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    view.state == NetState::Connected && FollowRole::from_slot(view.local_slot) == FollowRole::Guest
}

#[expect(
    unsafe_code,
    reason = "reads the game's NUL-terminated function name and calls the original through the trampoline"
)]
extern "system" fn call_function_detour(
    manager: *mut c_void,
    name: *const c_char,
    args: *mut c_void,
) -> u8 {
    if !name.is_null()
        && let Some(build) = HOOKED_BUILD.get()
    {
        // SAFETY: the game passes a NUL-terminated function name that outlives this call.
        let name_bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
        let watching = local_is_connected_guest();
        if watching {
            send_talker_writes(build, false);
        }
        let started = TRACKER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .start_call(name_bytes, net_clock_ms());
        if let Some(call) = started
            && call.from_talk_tag
        {
            log_line(&format!(
                "talk-callback: started hash={:016x}",
                call.name_hash
            ));
            if watching {
                *STORY_BEFORE_TALK_CALLBACK
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner) = read_live_arrays(build);
            }
        }
    }
    let original = ORIGINAL_CALL_FUNCTION.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original CallFunction, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, CallFunction>(original)(manager, name, args) }
}

#[expect(
    unsafe_code,
    reason = "reads the tag body and calls the original through the trampoline"
)]
extern "system" fn run_callback_detour(body: *const c_char, line_arg: i32) -> u8 {
    if !body.is_null() {
        // SAFETY: the game passes the NUL-terminated "Function,arg,..." tag body.
        let body_bytes = unsafe { CStr::from_ptr(body) }.to_bytes();
        TRACKER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_talk_tag_call(talk_tag_function_name(body_bytes), net_clock_ms());
    }
    let original = ORIGINAL_RUN_CALLBACK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original RunCallback, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, RunCallback>(original)(body, line_arg) }
}

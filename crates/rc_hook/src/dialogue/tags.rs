use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use rc_game::dialogue::{TALK_TAG_RUN_ALL_RVA, talk_tags_run_here};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::hooks::MIRRORING;
use crate::game_hooks::hook_address;

static ORIGINAL_RUN_ALL: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MIRROR_ACTION_RUNNING: AtomicBool = AtomicBool::new(false);
static SKIPPED_TAG_STRINGS: AtomicU32 = AtomicU32::new(0);

type RunAllTags = unsafe extern "system" fn(*const u8, i32);

pub(super) struct MirrorActionGuard;

impl MirrorActionGuard {
    pub(super) fn begin() -> Self {
        MIRROR_ACTION_RUNNING.store(true, Ordering::Release);
        Self
    }
}

impl Drop for MirrorActionGuard {
    fn drop(&mut self) {
        MIRROR_ACTION_RUNNING.store(false, Ordering::Release);
    }
}

pub(super) fn mirror_action_running() -> bool {
    MIRROR_ACTION_RUNNING.load(Ordering::Acquire)
}

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(unsafe_code, reason = "hooks TalkTag::RunAll by address")]
    // SAFETY: the RVA starts `void RunAll(char* tags, int lineArg)` in the verified build (see
    // docs off-hand/story-choices.md), and the replacement has the same signature.
    unsafe {
        hook_address(
            "TalkTag::RunAll",
            (base + u64::from(TALK_TAG_RUN_ALL_RVA)) as usize,
            run_all_tags_detour as *const () as *mut c_void,
            &ORIGINAL_RUN_ALL,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original TalkTag::RunAll through the trampoline MinHook built"
)]
extern "system" fn run_all_tags_detour(tags: *const u8, line_arg: i32) {
    if !talk_tags_run_here(
        MIRRORING.load(Ordering::Acquire),
        MIRROR_ACTION_RUNNING.load(Ordering::Acquire),
    ) {
        let skipped = SKIPPED_TAG_STRINGS.fetch_add(1, Ordering::Relaxed) + 1;
        log_line(&format!(
            "talk-tag: skipped on the watcher line_arg={line_arg} total={skipped}"
        ));
        return;
    }
    let original = ORIGINAL_RUN_ALL.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original RunAll, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, RunAllTags>(original)(tags, line_arg) }
}

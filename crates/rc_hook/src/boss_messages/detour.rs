use core::ffi::c_void;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use rc_game::boss_messages::{
    MAX_LOGGED_UNMIRRORED_NUMBERS, MirrorVerdict, POST_MESSAGE_RVA, RAISED_MESSAGE_MASK_BYTES,
    RAISED_MESSAGE_MASK_OFFSET, mirror_verdict,
};
use rc_game::cutscene_skip::TRIGGER_MANAGER_RVA;
use rc_game::stay_together::FollowRole;
use rc_game::{GameMemory, Replicant};
use rc_proto::{Message, ScriptMessage};

use crate::boss_messages::state::{
    CountedAs, HOOKED_BUILD, LAST_SENT_SEQ, LOGGED_UNMIRRORED, ORIGINAL_POST_MESSAGE, count,
};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState};
use crate::story_relay::shared_story_events;

type PostMessage = unsafe extern "system" fn(*mut c_void, u32);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks ScriptCallbacks::PostMessage by its address"
    )]
    // SAFETY: the RVA starts ScriptCallbacks::PostMessage (registry, message number) in the
    // verified build (docs off-hand/boss-fights.md); the replacement takes the same two
    // arguments and passes them on unchanged.
    unsafe {
        hook_address(
            "ScriptCallbacks::PostMessage",
            (build.module_base() + u64::from(POST_MESSAGE_RVA)) as usize,
            post_message_detour as *const () as *mut c_void,
            &ORIGINAL_POST_MESSAGE,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original PostMessage through the trampoline MinHook built"
)]
extern "system" fn post_message_detour(registry: *mut c_void, message: u32) {
    let original = ORIGINAL_POST_MESSAGE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original body, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, PostMessage>(original)(registry, message) };
    crate::rewind::note_posted_message(message);
    let _ = panic::catch_unwind(|| share_raised_message(message));
}

pub(super) fn raise_through_original(build: &VerifiedBuild, message: u32) -> bool {
    let original = ORIGINAL_POST_MESSAGE.load(Ordering::Acquire);
    if original.is_null() {
        return false;
    }
    let registry = (build.module_base() + u64::from(TRIGGER_MANAGER_RVA)) as *mut c_void;
    #[expect(
        unsafe_code,
        reason = "calls the original PostMessage through the trampoline MinHook built"
    )]
    // SAFETY: same signature as above; the registry address is the script trigger manager the
    // game's own callers pass; runs on the game thread in the script tick, where scripts post too.
    unsafe {
        core::mem::transmute::<*mut c_void, PostMessage>(original)(registry, message);
    }
    true
}

fn share_raised_message(message: u32) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    if view.state != NetState::Connected
        || !shared_story_events()
        || FollowRole::from_slot(view.local_slot) != FollowRole::Host
    {
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    match mirror_verdict(message) {
        MirrorVerdict::Mirror => {}
        MirrorVerdict::NeverMirrored => {
            count(CountedAs::Skipped);
            log_unmirrored_with_entry(build, game.memory(), message, "a player post");
            return;
        }
        MirrorVerdict::MixedPoster => {
            count(CountedAs::Skipped);
            log_unmirrored_with_entry(build, game.memory(), message, "a mixed poster");
            return;
        }
        MirrorVerdict::OutsideRange => {
            count(CountedAs::Skipped);
            log_unmirrored_with_entry(
                build,
                game.memory(),
                message,
                "never seen in the post-site scan",
            );
            return;
        }
    }
    let Some(phase) = crate::boss_messages::tick::current_phase_index(&game) else {
        count(CountedAs::Skipped);
        return;
    };
    let seq = LAST_SENT_SEQ
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    net.send_relayed(Message::ScriptMessage(ScriptMessage {
        phase,
        message,
        seq,
    }));
    count(CountedAs::Sent);
    crate::boss_messages::fight::raise(phase);
    log_line(&format!(
        "boss message: host raised {message}, sent for phase {phase} as number {seq}"
    ));
}

fn log_unmirrored_with_entry(
    build: &VerifiedBuild,
    memory: &InProcessMemory,
    message: u32,
    class: &str,
) {
    let Ok(index) = usize::try_from(message) else {
        return;
    };
    if index >= RAISED_MESSAGE_MASK_BYTES * 8 {
        return;
    }
    let mask_address = build.module_base()
        + u64::from(TRIGGER_MANAGER_RVA)
        + RAISED_MESSAGE_MASK_OFFSET
        + (index / 8) as u64;
    let Ok(byte) = memory.read_u8(mask_address) else {
        return;
    };
    if byte & (1 << (index % 8)) == 0 {
        return;
    }
    let mut logged = LOGGED_UNMIRRORED
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if logged.contains(&message) || logged.len() >= MAX_LOGGED_UNMIRRORED_NUMBERS {
        return;
    }
    logged.push(message);
    drop(logged);
    log_line(&format!(
        "boss message: host raised {message} (has a registered entry, {class}, not sent)"
    ));
}

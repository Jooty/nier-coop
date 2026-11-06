use core::ffi::c_void;
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};

use rc_game::GameMemory;
use rc_game::npcs::{NPC_TALK_COMMAND_IDS, refuses_talk_start, talk_busy_notice};
use rc_game::offsets::{HUMAN_NPC_COMMAND_FUNCTION_RVA, NPC_COMMAND_ID_OFFSET};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_COMMAND: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static PARTNER_TALKS: LazyLock<Mutex<PartnerTalkSet>> = LazyLock::new(Mutex::default);

type NpcCommand = unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void) -> u64;

#[derive(Default)]
struct PartnerTalkSet {
    actors: HashSet<u64>,
    partner_name: String,
}

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the villager AI command dispatcher by address"
    )]
    // SAFETY: the RVA starts the human NPC AI command dispatcher (this, actor, command; command id
    // at +0x10) in the verified build; the replacement has the same three arguments and passes
    // them on unchanged unless it refuses a talk start.
    unsafe {
        hook_address(
            "CActorHumanNpc_AiCommand",
            (build.module_base() + u64::from(HUMAN_NPC_COMMAND_FUNCTION_RVA)) as usize,
            npc_command_detour as *const () as *mut c_void,
            &ORIGINAL_COMMAND,
        );
    }
}

pub(super) fn set_partner_talks(actors: HashSet<u64>, partner_name: &str) {
    let mut talks = PARTNER_TALKS.lock().unwrap_or_else(PoisonError::into_inner);
    talks.actors = actors;
    if talks.partner_name != partner_name {
        partner_name.clone_into(&mut talks.partner_name);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original AI command dispatcher through the trampoline MinHook built"
)]
extern "system" fn npc_command_detour(
    this: *mut c_void,
    actor: *mut c_void,
    command: *mut c_void,
) -> u64 {
    if panic::catch_unwind(|| refuse_talk(actor as u64, command as u64)).unwrap_or(false) {
        return 0;
    }
    let original = ORIGINAL_COMMAND.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original dispatcher, which has this exact
    // signature.
    unsafe { core::mem::transmute::<*mut c_void, NpcCommand>(original)(this, actor, command) }
}

fn refuse_talk(actor: u64, command: u64) -> bool {
    let Some(build) = HOOKED_BUILD.get() else {
        return false;
    };
    let memory = InProcessMemory::new(build);
    let Ok(command_id) = memory.read_i32(command + NPC_COMMAND_ID_OFFSET) else {
        return false;
    };
    if !NPC_TALK_COMMAND_IDS.contains(&command_id) {
        return false;
    }
    let talks = PARTNER_TALKS.lock().unwrap_or_else(PoisonError::into_inner);
    let partner_talking = talks.actors.contains(&actor);
    if !refuses_talk_start(command_id, partner_talking) {
        log_line(&format!(
            "npcs: talk command {command_id:#x} on {actor:#x} allowed (the other player talks to it: {partner_talking}, villagers held for the other player's talk: {})",
            talks.actors.len()
        ));
        return false;
    }
    // The event log drops a repeat of the same text within its duplicate window.
    post_event(&talk_busy_notice(&talks.partner_name));
    log_line(&format!(
        "npcs: refused talk command {command_id:#x} on {actor:#x}, the other player is talking to it"
    ));
    true
}

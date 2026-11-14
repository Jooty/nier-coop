use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::message_handlers::{
    HandlerRegistered, MessageHandlers, SET_MESSAGE_CALLBACK_NATIVE_RVA, handler_name_from_parts,
    is_player_local_handler, message_id_from_parts, message_id_tag_range, message_id_value_range,
    message_name_tag_range, message_name_text_range,
};
use replicant_game::{GameMemory, PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{hook_address, note_native};
use crate::memory::InProcessMemory;
use crate::story_relay::state::HOOKED_BUILD;

static HANDLERS: Mutex<MessageHandlers> = Mutex::new(MessageHandlers::new());
static ORIGINAL_SET_MESSAGE_CALLBACK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the _SetMessageCallback script native by address to learn which handlers a pack registers"
    )]
    // SAFETY: the RVA starts _SetMessageCallback in the verified build (binding table 0x1306710,
    // thunk 0x6E1720), which takes the Lua argument block as its only argument.
    unsafe {
        hook_address(
            "LuaNative::SetMessageCallback",
            (build.module_base() + u64::from(SET_MESSAGE_CALLBACK_NATIVE_RVA)) as usize,
            set_message_callback_detour as *const () as *mut c_void,
            &ORIGINAL_SET_MESSAGE_CALLBACK,
        );
    }
}

pub(crate) fn is_host_driven_handler(phase: u16, name: &str) -> bool {
    HANDLERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_host_driven(phase, name)
}

pub(crate) fn is_message_handler(phase: u16, name: &str) -> bool {
    HANDLERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_registered(phase, name)
}

pub(crate) fn is_player_local_message_handler(
    phase: u16,
    name: &str,
    more_local_message_ids: &[i32],
) -> bool {
    HANDLERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_player_local(phase, name, more_local_message_ids)
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
extern "system" fn set_message_callback_detour(args: *mut c_void) {
    let args_address = args as u64;
    let _ = panic::catch_unwind(|| {
        note_native("_SetMessageCallback");
        note_registration(args_address);
    });
    let trampoline = ORIGINAL_SET_MESSAGE_CALLBACK.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, LuaNative>(trampoline)(args) };
    }
}

fn note_registration(args_address: u64) {
    if args_address == 0 {
        return;
    }
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let Some((name, message_id)) = read_registration(&memory, args_address) else {
        return;
    };
    let game = Replicant::new(InProcessMemory::new(build));
    let pack = game.current_phase().ok();
    if let Some(pack) = pack.as_deref() {
        crate::rule_rooms::note_registration(pack, &name);
    }
    let phase = pack
        .as_deref()
        .and_then(PhaseEntry::index_of)
        .and_then(|index| u16::try_from(index).ok());
    let Some(phase) = phase else {
        return;
    };
    crate::fight_sync::note_registration();
    let registered = HANDLERS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .register(phase, &name, message_id);
    match registered {
        HandlerRegistered::New => log_line(&format!(
            "message handler: {name} (message {message_id}) recorded for phase {phase}{}",
            if is_player_local_handler(&name, message_id) {
                ", player-local, each game runs its own"
            } else {
                ""
            }
        )),
        HandlerRegistered::TableFull => log_line(&format!(
            "message handler: {name} not recorded, the table is full"
        )),
        HandlerRegistered::Known => {}
    }
}

fn read_registration(memory: &InProcessMemory, args_address: u64) -> Option<(String, i32)> {
    let name_tag = read_range(memory, args_address, message_name_tag_range())?;
    let name_text = read_range(memory, args_address, message_name_text_range())?;
    let id_tag = read_range(memory, args_address, message_id_tag_range())?;
    let id_value = read_range(memory, args_address, message_id_value_range())?;
    let name = handler_name_from_parts(&name_tag, &name_text)?;
    let message_id = message_id_from_parts(&id_tag, &id_value)?;
    Some((name, message_id))
}

fn read_range(
    memory: &InProcessMemory,
    args_address: u64,
    range: core::ops::Range<usize>,
) -> Option<Vec<u8>> {
    let mut bytes = vec![0_u8; range.len()];
    memory
        .read(args_address + range.start as u64, &mut bytes)
        .ok()?;
    Some(bytes)
}

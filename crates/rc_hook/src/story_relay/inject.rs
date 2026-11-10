use core::ffi::{c_char, c_void};
use std::ffi::CString;

use rc_game::cutscene_skip::TRIGGER_MANAGER_RVA;
use rc_game::story_relay::{
    INJECTED_CALL_PRIORITY, MARK_QUEUED_CALL_READY_RVA, QUEUE_SCRIPT_CALL_RVA,
    QUEUED_CALL_ARGUMENTS_OFFSET, QUEUED_CALL_FREE_STATE, QUEUED_CALL_READY_STATE,
    write_args_block,
};
use rc_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::story_relay::call_gate::relay_call_cancelled;

type QueueScriptCall =
    unsafe extern "system" fn(*mut c_void, *const c_char, *const c_void, i32) -> *mut c_void;
type MarkQueuedCallReady = unsafe extern "system" fn(*mut c_void, *mut c_void);

pub(super) fn queue_call_now(build: &VerifiedBuild, call: &StoryCall) -> Option<u64> {
    let queued = queue_call_unmarked(build, call)?;
    crate::fight_sync::partner_call_queued(&call.function);
    crate::game_hooks::note_host_led_handler(&call.function);
    Some(queued)
}

#[expect(
    unsafe_code,
    reason = "calls the game's own script call queue and its ready marker by address"
)]
pub(crate) fn queue_call_unmarked(build: &VerifiedBuild, call: &StoryCall) -> Option<u64> {
    let name = CString::new(call.function.as_str()).ok()?;
    let block = write_args_block(&call.args);
    let block_pointer: *const c_void = if call.args.is_empty() {
        core::ptr::null()
    } else {
        block.as_ptr().cast()
    };
    let base = build.module_base();
    let manager = (base + u64::from(TRIGGER_MANAGER_RVA)) as *mut c_void;
    let queue = (base + u64::from(QUEUE_SCRIPT_CALL_RVA)) as *const ();
    let mark_ready = (base + u64::from(MARK_QUEUED_CALL_READY_RVA)) as *const ();
    // SAFETY: both RVAs start TriggerManager::QueueScriptCall and MarkEventReady in the verified
    // build with these signatures; the name (checked by the wire validation) and the argument
    // block outlive the call and are copied by the game; runs on the game thread in the script tick.
    let slot = unsafe {
        core::mem::transmute::<*const (), QueueScriptCall>(queue)(
            manager,
            name.as_ptr(),
            block_pointer,
            INJECTED_CALL_PRIORITY,
        )
    };
    if slot.is_null() {
        return None;
    }
    let _own_arm = crate::fight_sync::OwnArm::begin();
    // SAFETY: the slot pointer was just returned by the queue call above.
    unsafe { core::mem::transmute::<*const (), MarkQueuedCallReady>(mark_ready)(manager, slot) };
    Some(slot as u64 + QUEUED_CALL_ARGUMENTS_OFFSET)
}

#[expect(
    unsafe_code,
    reason = "clears the ready state of a queue slot the game has not popped yet"
)]
pub(super) fn cancel_queued_call(queued_arguments: u64) {
    let slot = (queued_arguments - QUEUED_CALL_ARGUMENTS_OFFSET) as *mut i32;
    // SAFETY: the pointer is the queue slot this hook queued itself (argument pointer minus the
    // argument offset), inside the game's fixed script manager object; this runs on the game
    // thread in the callback pop. A slot that is no longer ready is left alone.
    unsafe {
        if slot.read() == QUEUED_CALL_READY_STATE {
            slot.write(QUEUED_CALL_FREE_STATE);
            relay_call_cancelled(queued_arguments);
            log_line("story event: queued copy cancelled, the natural fire runs instead");
        }
    }
}

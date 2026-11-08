use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::GameMemory;
use rc_game::position_log::{
    POSITION_NATIVES, describe_arguments, native_argument_offset, native_argument_value_offset,
};
use rc_game::story_jump::{NATIVE_ARG_INTEGER_TYPE_TAG, NATIVE_ARG_REAL_TYPE_TAG};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::position_log::own_call::own_call_is_running;
use crate::story_writes::calls::running_call;

static ORIGINAL_SET_PLAYER_POSITION: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_PLAYER_POSITION_TO_POINT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SET_UNIQUE_ACTOR_POSITION: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BUILD: OnceLock<VerifiedBuild> = OnceLock::new();

type PositionNativeFn = unsafe extern "system" fn(*mut c_void);

pub(crate) fn install(build: VerifiedBuild) {
    let _ = BUILD.set(build);
    let targets: [(&AtomicPtr<c_void>, *mut c_void); 3] = [
        (
            &ORIGINAL_SET_PLAYER_POSITION,
            set_player_position_detour as *const () as *mut c_void,
        ),
        (
            &ORIGINAL_SET_PLAYER_POSITION_TO_POINT,
            set_player_position_to_point_detour as *const () as *mut c_void,
        ),
        (
            &ORIGINAL_SET_UNIQUE_ACTOR_POSITION,
            set_unique_actor_position_detour as *const () as *mut c_void,
        ),
    ];
    for (native, (original, replacement)) in POSITION_NATIVES.iter().zip(targets) {
        #[expect(
            unsafe_code,
            reason = "hooks a script position native by address to log its calls"
        )]
        // SAFETY: each RVA starts the native of that name in the verified build (docs
        // symbols.tsv, found by the binding-table walk) and the replacement has its signature.
        unsafe {
            hook_address(
                native.name,
                (build.module_base() + u64::from(native.rva)) as usize,
                replacement,
                original,
            );
        }
    }
}

extern "system" fn set_player_position_detour(args: *mut c_void) {
    let _ = panic::catch_unwind(|| log_call(0, args as u64));
    forward(&ORIGINAL_SET_PLAYER_POSITION, args);
}

extern "system" fn set_player_position_to_point_detour(args: *mut c_void) {
    let _ = panic::catch_unwind(|| log_call(1, args as u64));
    forward(&ORIGINAL_SET_PLAYER_POSITION_TO_POINT, args);
}

extern "system" fn set_unique_actor_position_detour(args: *mut c_void) {
    let _ = panic::catch_unwind(|| log_call(2, args as u64));
    forward(&ORIGINAL_SET_UNIQUE_ACTOR_POSITION, args);
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline MinHook built"
)]
fn forward(original: &AtomicPtr<c_void>, args: *mut c_void) {
    let trampoline = original.load(Ordering::Acquire);
    if trampoline.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, PositionNativeFn>(trampoline)(args) };
}

fn log_call(native_index: usize, args_address: u64) {
    let Some(build) = BUILD.get() else {
        return;
    };
    let native = POSITION_NATIVES[native_index];
    let memory = InProcessMemory::new(build);
    let mut arguments = Vec::new();
    for index in 0..native.max_arguments {
        let Ok(tag_bytes) =
            memory.read_array::<4>(args_address + native_argument_offset(index) as u64)
        else {
            break;
        };
        let tag = u32::from_le_bytes(tag_bytes);
        if tag != NATIVE_ARG_INTEGER_TYPE_TAG && tag != NATIVE_ARG_REAL_TYPE_TAG {
            break;
        }
        let Ok(bits) = memory.read_u64(args_address + native_argument_value_offset(index) as u64)
        else {
            break;
        };
        arguments.push((tag, bits));
    }
    let caller = running_call().map_or_else(
        || "none".to_owned(),
        |call| format!("{:#x}", call.name_hash),
    );
    let origin = if own_call_is_running() {
        "ours"
    } else {
        "script"
    };
    log_line(&format!(
        "position: {} args {} from call {caller} ({origin})",
        native.name,
        describe_arguments(&arguments)
    ));
}

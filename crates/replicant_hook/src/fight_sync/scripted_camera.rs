use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::fight_sync::{
    ScriptedCameraNative, camera_call_line, forward_of_world_matrix, read_native_numbers,
};
use replicant_game::marker::read_camera_state;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{BUILD, current_phase_index};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

const CAMERA_NATIVE_COUNT: usize = 5;

static ORIGINALS: [AtomicPtr<c_void>; CAMERA_NATIVE_COUNT] =
    [const { AtomicPtr::new(ptr::null_mut()) }; CAMERA_NATIVE_COUNT];

type CameraNative = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    let detours: [*mut c_void; CAMERA_NATIVE_COUNT] = [
        set_fixed_camera_detour as *const () as *mut c_void,
        change_camera_detour as *const () as *mut c_void,
        direction_fixed_detour as *const () as *mut c_void,
        distance_offset_detour as *const () as *mut c_void,
        direction_with_actor_group_detour as *const () as *mut c_void,
    ];
    for ((native, detour), original) in ScriptedCameraNative::ALL
        .into_iter()
        .zip(detours)
        .zip(&ORIGINALS)
    {
        #[expect(unsafe_code, reason = "hooks a camera script native by its address")]
        // SAFETY: each RVA starts the named native's body in the verified build (docs
        // symbols.tsv): one pointer in (its argument block), nothing out. The detours match it.
        unsafe {
            hook_address(
                native.name(),
                (build.module_base() + u64::from(native.rva())) as usize,
                detour,
                original,
            );
        }
    }
}

fn read_view(build: &VerifiedBuild) -> (Option<[f32; 3]>, Option<i32>) {
    match read_camera_state(&InProcessMemory::new(build)) {
        Ok(Some(camera)) => (
            forward_of_world_matrix(&camera.world_matrix),
            Some(camera.active_index),
        ),
        _ => (None, None),
    }
}

fn camera_native_ran(native: ScriptedCameraNative, args: *mut c_void) {
    let trampoline = ORIGINALS[usize::from(native.code() - 1)].load(Ordering::Acquire);
    let args_address = args as u64;
    let before = panic::catch_unwind(|| note_call_before(native, args_address)).ok();
    if !trampoline.is_null() {
        call_original(trampoline, args);
    }
    if let Some(Some(line)) = before {
        let _ = panic::catch_unwind(|| log_line(&line));
    }
}

fn note_call_before(native: ScriptedCameraNative, args_address: u64) -> Option<String> {
    let build = BUILD.get()?;
    let memory = InProcessMemory::new(build);
    let numbers = read_native_numbers(&memory, args_address, native.argument_count());
    let (forward, active_camera) = read_view(build);
    let in_window = current_phase_index(build).is_some_and(crate::fight_sync::covers_phase);
    Some(camera_call_line(
        native,
        &numbers,
        forward,
        active_camera,
        in_window,
    ))
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
fn call_original(trampoline: *mut c_void, args: *mut c_void) {
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, CameraNative>(trampoline)(args) };
}

extern "system" fn set_fixed_camera_detour(args: *mut c_void) {
    camera_native_ran(ScriptedCameraNative::SetFixedCamera, args);
}

extern "system" fn change_camera_detour(args: *mut c_void) {
    camera_native_ran(ScriptedCameraNative::ChangeCamera, args);
}

extern "system" fn direction_fixed_detour(args: *mut c_void) {
    camera_native_ran(ScriptedCameraNative::SetFreeCameraDirectionFixed, args);
}

extern "system" fn distance_offset_detour(args: *mut c_void) {
    camera_native_ran(ScriptedCameraNative::SetFreeCameraDistanceOffset, args);
}

extern "system" fn direction_with_actor_group_detour(args: *mut c_void) {
    camera_native_ran(
        ScriptedCameraNative::ResetFreeCameraDirectionWithActorGroup,
        args,
    );
}

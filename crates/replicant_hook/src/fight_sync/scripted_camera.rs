use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::fight_sync::{
    GUEST_SET_DIRECTION_BLEND_SECONDS, GUEST_SET_DIRECTION_EASE, GuestCameraFollow,
    GuestCameraStep, GuestCameraView, HostCameraSampler, SNOW_CAMERA_INDEX, ScriptedCameraNative,
    camera_call_line, forward_of_world_matrix, guest_camera_step_line, read_native_numbers,
    settled_camera_line,
};
use replicant_game::marker::read_camera_state;
use replicant_game::side_view::CHANGE_CAMERA_NATIVE_RVA;
use replicant_game::stay_together::FollowRole;
use replicant_proto::{FightCameraAngle, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::radio_waits::in_event_scene;
use crate::fight_sync::state::{BUILD, current_phase_index, here};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::stay_together::pause_menu_open;
use crate::story_jump::{NativeArg, call_native_with};

const CAMERA_NATIVE_COUNT: usize = 5;
const CHANGE_CAMERA_TO_SNOW_BLEND_SECONDS: f64 = 0.0;
const CHANGE_CAMERA_BLEND_MODE: i32 = 2;

static ORIGINALS: [AtomicPtr<c_void>; CAMERA_NATIVE_COUNT] =
    [const { AtomicPtr::new(ptr::null_mut()) }; CAMERA_NATIVE_COUNT];
static SAMPLER: LazyLock<Mutex<HostCameraSampler>> = LazyLock::new(Mutex::default);
static FOLLOW: LazyLock<Mutex<GuestCameraFollow>> = LazyLock::new(Mutex::default);

thread_local! {
    static APPLYING_HOST_ANGLE: Cell<bool> = const { Cell::new(false) };
}

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

pub(super) fn tick(build: &VerifiedBuild) {
    let Some(here) = here(build) else {
        return;
    };
    let Some(net) = net::handle() else {
        return;
    };
    let now_ms = net_clock_ms();
    let received = net.take_relayed(|message| matches!(message, Message::FightCameraAngle(_)));
    match here.role {
        FollowRole::Host => {
            let due = SAMPLER
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .poll(now_ms, pause_menu_open(build));
            if let Some(native) = due {
                sample_and_send(build, native);
            }
        }
        FollowRole::Guest => {
            let mut follow = FOLLOW.lock().unwrap_or_else(PoisonError::into_inner);
            for message in received {
                if let Message::FightCameraAngle(angle) = message {
                    follow.receive(angle, now_ms);
                }
            }
            let (forward, active_camera) = read_view(build);
            let view = GuestCameraView {
                menu_open: pause_menu_open(build),
                in_event_scene: in_event_scene(build),
                phase: current_phase_index(build),
                forward,
                active_camera: active_camera.unwrap_or(-1),
            };
            let step = follow.poll(now_ms, &view);
            drop(follow);
            if let Some(step) = step {
                log_line(&guest_camera_step_line(&step, forward));
                if let GuestCameraStep::Apply {
                    angle,
                    change_to_snow_camera,
                    ..
                } = step
                {
                    apply_host_angle(build, &angle, change_to_snow_camera);
                }
            }
        }
        FollowRole::Unknown => {}
    }
}

fn sample_and_send(build: &VerifiedBuild, native: ScriptedCameraNative) {
    let (forward, active_camera) = read_view(build);
    let phase = current_phase_index(build);
    let in_window = phase.is_some_and(crate::fight_sync::covers_phase);
    let angle = forward
        .zip(active_camera)
        .zip(phase)
        .map(|((forward, active_camera), phase)| FightCameraAngle {
            phase,
            native: native.code(),
            forward,
            active_camera,
        })
        .filter(|angle| in_window && angle.validate().is_ok());
    log_line(&settled_camera_line(
        native,
        forward,
        active_camera,
        angle.is_some(),
    ));
    if let (Some(angle), Some(net)) = (angle, net::handle()) {
        net.send_relayed(Message::FightCameraAngle(angle));
    }
}

fn apply_host_angle(build: &VerifiedBuild, angle: &FightCameraAngle, change_to_snow_camera: bool) {
    APPLYING_HOST_ANGLE.with(|cell| cell.set(true));
    let [x, y, z] = angle.forward.map(f64::from);
    let [ease_in, ease_rate, ease_out] = GUEST_SET_DIRECTION_EASE;
    // The native looks from its first point to its second; the order follows the fight script.
    call_native_with(
        build,
        ScriptedCameraNative::SetFreeCameraDirectionFixed.rva(),
        &[
            NativeArg::Real(0.0),
            NativeArg::Real(0.0),
            NativeArg::Real(0.0),
            NativeArg::Real(x),
            NativeArg::Real(y),
            NativeArg::Real(z),
            NativeArg::Integer(0),
            NativeArg::Real(GUEST_SET_DIRECTION_BLEND_SECONDS),
            NativeArg::Real(ease_in),
            NativeArg::Real(ease_rate),
            NativeArg::Real(ease_out),
        ],
    );
    if change_to_snow_camera {
        call_native_with(
            build,
            CHANGE_CAMERA_NATIVE_RVA,
            &[
                NativeArg::Integer(SNOW_CAMERA_INDEX),
                NativeArg::Real(CHANGE_CAMERA_TO_SNOW_BLEND_SECONDS),
                NativeArg::Integer(CHANGE_CAMERA_BLEND_MODE),
            ],
        );
    }
    APPLYING_HOST_ANGLE.with(|cell| cell.set(false));
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
    if APPLYING_HOST_ANGLE.with(Cell::get) {
        return None;
    }
    let build = BUILD.get()?;
    let now_ms = net_clock_ms();
    let role = here(build).map(|here| here.role);
    match role {
        Some(FollowRole::Host) => SAMPLER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_call(native, now_ms),
        Some(FollowRole::Guest) => FOLLOW
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_own_call(now_ms),
        _ => {}
    }
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

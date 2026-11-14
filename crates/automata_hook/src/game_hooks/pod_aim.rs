use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use automata_game::address::{
    CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION, GameAddress, POD_SHOT_UPDATE_FUNCTION,
};
use automata_game::partner::{
    AimRayPoints, CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START, HERO_POD_AIM_IS_SENT_FOR_MS,
    POD_SHOT_UPDATE_FUNCTION_START, PodAim, code_starts_with,
};
use nt_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_slot_body;

const SCREEN_DEPTH_INDEX_IN_A_SCREEN_POINT: usize = 2;
const W_OF_A_WORLD_POINT: f32 = 1.0;

static ORIGINAL_POD_SHOT_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SCREEN_POINT_TO_WORLD: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static POD_AIM_CALLS: OnceLock<GameCalls> = OnceLock::new();
static PARTNER_POD_AIM: Mutex<Option<PodAim>> = Mutex::new(None);
static HERO_POD_AIM: Mutex<Option<(PodAim, Instant)>> = Mutex::new(None);
static FIRST_PARTNER_RAY_POINT_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_HERO_AIM_LOGGED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static POD_SHOT_THAT_RUNS: Cell<PodShotOf> = const { Cell::new(PodShotOf::AnotherBody) };
    static HERO_RAY_POINTS: Cell<AimRayPoints> = const { Cell::new(AimRayPoints::new()) };
}

type PodShotUpdate = unsafe extern "system" fn(usize);
type ScreenPointToWorld = unsafe extern "system" fn(usize, usize, usize);

#[derive(Debug, Clone, Copy, PartialEq)]
enum PodShotOf {
    AnotherBody,
    Hero,
    Partner(PodAim),
}

/// Makes the partner body's pod shot aim along the other player's ray and records the hero's own
/// ray for sending; false, with the pod shot left as the game runs it, when a function start differs.
pub fn install_pod_aim(calls: GameCalls) -> bool {
    let starts_match = start_is_the_noted_one(
        &calls,
        CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION,
        &CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION_START,
    ) && start_is_the_noted_one(
        &calls,
        POD_SHOT_UPDATE_FUNCTION,
        &POD_SHOT_UPDATE_FUNCTION_START,
    );
    let hooked = starts_match
        && POD_AIM_CALLS.set(calls).is_ok()
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the camera's screen point to world point; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the function only the pod shot calls, of the verified build,
            // and its start was just checked; the replacement forwards all three arguments.
            unsafe {
                nt_hook::detour::hook_address(
                    "camera screen point to world point (the partner's pod aim)",
                    calls.address_value(CAMERA_SCREEN_POINT_TO_WORLD_FUNCTION) as usize,
                    screen_point_to_world_detour as *const () as *mut c_void,
                    &ORIGINAL_SCREEN_POINT_TO_WORLD,
                )
            }
        }
        && {
            #[expect(
                unsafe_code,
                reason = "hooks a body's pod shot update; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the pod shot update of the verified build and its start was
            // just checked; the replacement forwards the one argument.
            unsafe {
                nt_hook::detour::hook_address(
                    "player body pod shot update (whose pod shot runs)",
                    calls.address_value(POD_SHOT_UPDATE_FUNCTION) as usize,
                    pod_shot_update_detour as *const () as *mut c_void,
                    &ORIGINAL_POD_SHOT_UPDATE,
                )
            }
        };
    if !hooked {
        log_line(
            "pod aim: the pod shot is not hooked, the partner's pod aims along the local camera as before and no aim is sent",
        );
    }
    hooked
}

/// The other player's aim for the held body's pod shot; none leaves that pod shot as the game runs it.
pub fn set_partner_pod_aim(aim: Option<PodAim>) {
    if let Ok(mut held) = PARTNER_POD_AIM.lock() {
        *held = aim.and_then(PodAim::checked);
    }
}

/// The ray the local hero's own pod shot aimed along, while it is fresh enough to send.
pub fn recorded_pod_aim() -> Option<PodAim> {
    let (aim, recorded_at) = (*HERO_POD_AIM.lock().ok()?)?;
    (recorded_at.elapsed() <= Duration::from_millis(HERO_POD_AIM_IS_SENT_FOR_MS)).then_some(aim)
}

#[expect(
    unsafe_code,
    reason = "calls the original pod shot update through the trampoline MinHook built"
)]
extern "system" fn pod_shot_update_detour(body: usize) {
    let original = ORIGINAL_POD_SHOT_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let whose = whose_pod_shot(body);
    if whose == PodShotOf::Hero {
        HERO_RAY_POINTS.set(AimRayPoints::new());
    }
    let before = POD_SHOT_THAT_RUNS.replace(whose);
    // SAFETY: the trampoline continues into the original function with the argument received.
    unsafe { core::mem::transmute::<*mut c_void, PodShotUpdate>(original)(body) };
    POD_SHOT_THAT_RUNS.set(before);
    if whose == PodShotOf::Hero {
        keep_the_heros_aim(HERO_RAY_POINTS.get().aim());
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original screen point to world point through the trampoline MinHook built"
)]
extern "system" fn screen_point_to_world_detour(
    camera: usize,
    world_point: usize,
    screen_point: usize,
) {
    let original = ORIGINAL_SCREEN_POINT_TO_WORLD.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    let whose = POD_SHOT_THAT_RUNS.get();
    if let PodShotOf::Partner(aim) = whose
        && gave_the_partners_ray_point(aim, world_point, screen_point)
    {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe {
        core::mem::transmute::<*mut c_void, ScreenPointToWorld>(original)(
            camera,
            world_point,
            screen_point,
        );
    }
    if whose == PodShotOf::Hero {
        note_the_heros_ray_point(world_point, screen_point);
    }
}

fn start_is_the_noted_one(calls: &GameCalls, function: GameAddress, noted_start: &[u8]) -> bool {
    calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start))
}

fn whose_pod_shot(body: usize) -> PodShotOf {
    let Some(calls) = POD_AIM_CALLS.get() else {
        return PodShotOf::AnotherBody;
    };
    // The local player's own body is asked first: its pod shot is never given another ray.
    if calls.current_hero() == body {
        return PodShotOf::Hero;
    }
    let aim = PARTNER_POD_AIM.lock().ok().and_then(|held| *held);
    match (aim, guarded_slot_body()) {
        (Some(aim), Some((_, held_body))) if held_body == body => PodShotOf::Partner(aim),
        _ => PodShotOf::AnotherBody,
    }
}

fn screen_depth_of(calls: &GameCalls, screen_point: usize) -> Option<f32> {
    let point: [f32; 4] = calls.read_f32s_at(screen_point as u64).ok()?;
    Some(point[SCREEN_DEPTH_INDEX_IN_A_SCREEN_POINT])
}

fn gave_the_partners_ray_point(aim: PodAim, world_point: usize, screen_point: usize) -> bool {
    let Some(calls) = POD_AIM_CALLS.get() else {
        return false;
    };
    let Some(point) = screen_depth_of(calls, screen_point).and_then(|depth| aim.ray_point(depth))
    else {
        return false;
    };
    let written = calls
        .write_f32s_at(
            world_point as u64,
            &[point[0], point[1], point[2], W_OF_A_WORLD_POINT],
        )
        .is_ok();
    if written && !FIRST_PARTNER_RAY_POINT_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(&format!(
            "pod aim: the partner body's pod shot got the other player's ray in place of the local camera's (eye {:?}, direction {:?})",
            aim.eye, aim.direction
        ));
    }
    written
}

fn note_the_heros_ray_point(world_point: usize, screen_point: usize) {
    let Some(calls) = POD_AIM_CALLS.get() else {
        return;
    };
    let Some(depth) = screen_depth_of(calls, screen_point) else {
        return;
    };
    let Ok(point) = calls.read_f32s_at::<4>(world_point as u64) else {
        return;
    };
    let mut points = HERO_RAY_POINTS.get();
    points.note(depth, [point[0], point[1], point[2]]);
    HERO_RAY_POINTS.set(points);
}

fn keep_the_heros_aim(aim: Option<PodAim>) {
    if let Ok(mut kept) = HERO_POD_AIM.lock() {
        *kept = aim.map(|aim| (aim, Instant::now()));
    }
    if let Some(aim) = aim
        && !FIRST_HERO_AIM_LOGGED.swap(true, Ordering::Relaxed)
    {
        log_line(&format!(
            "pod aim: first ray of the hero's own pod shot recorded for sending (eye {:?}, direction {:?})",
            aim.eye, aim.direction
        ));
    }
}

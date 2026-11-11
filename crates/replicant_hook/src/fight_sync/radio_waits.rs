use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::fight_sync::{
    GuestRadioWaitCall, POLLED_NATIVE_STATE_OFFSET, RADIO_ABORT_FLAG_BIT,
    RADIO_ABORT_FLAG_WORD_RVA, RADIO_ACTIVE_BYTE_OFFSET, RADIO_BUSY_BYTE_OFFSET,
    RADIO_LINE_FLAG_OFFSET, RADIO_LINE_STATE_OFFSET, RADIO_LINE_TIME_OFFSET, RADIO_MANAGER_RVA,
    RADIO_STOP_RVA, RadioState, START_RADIO_WAIT_RVA, WAIT_NATIVE_WAIT_RVA,
    format_radio_line_under_menu, format_radio_wait_release,
};
use replicant_game::marker::EVENT_SCENE_BYTE_RVA;
use replicant_game::stay_together::FollowRole;
use replicant_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::partner_in_cutscene;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{BUILD, GUEST, HOST, here};
use crate::game_hooks::{hook_address, note_yielding_native, slot_holder_name};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::stay_together::{main_menu_open, pause_menu_open};

static ORIGINAL_START_RADIO_WAIT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_WAIT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LINE_TIME_AT_LAST_GUEST_POLL: Mutex<Option<f32>> = Mutex::new(None);

type PolledNative = unsafe extern "system" fn(*mut c_void) -> u8;
type RadioStop = unsafe extern "system" fn(*mut c_void, u8);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the polled waits of two script natives by address"
    )]
    // SAFETY: each RVA starts the wait the script step polls for that native in the verified
    // build (`_StartRadio` 0x70A6B0, `_Wait` 0x70BDA0; docs symbols.tsv): one pointer in (the
    // script manager + 0xD8), a byte out (non-zero while it waits). The replacements match it.
    unsafe {
        hook_address(
            "LuaNative:_StartRadio",
            (base + u64::from(START_RADIO_WAIT_RVA)) as usize,
            start_radio_wait_detour as *const () as *mut c_void,
            &ORIGINAL_START_RADIO_WAIT,
        );
        hook_address(
            "LuaNative:_Wait",
            (base + u64::from(WAIT_NATIVE_WAIT_RVA)) as usize,
            wait_detour as *const () as *mut c_void,
            &ORIGINAL_WAIT,
        );
    }
}

pub(super) fn read_radio_state(build: &VerifiedBuild) -> Option<RadioState> {
    let memory = InProcessMemory::new(build);
    let base = build.module_base();
    let manager = base + u64::from(RADIO_MANAGER_RVA);
    let abort_word = memory
        .read_i32(base + u64::from(RADIO_ABORT_FLAG_WORD_RVA))
        .ok()?
        .cast_unsigned();
    Some(RadioState {
        abort_flag: abort_word & RADIO_ABORT_FLAG_BIT != 0,
        event_scene: memory
            .read_u8(base + u64::from(EVENT_SCENE_BYTE_RVA))
            .ok()?,
        active: memory.read_u8(manager + RADIO_ACTIVE_BYTE_OFFSET).ok()?,
        busy: memory.read_u8(manager + RADIO_BUSY_BYTE_OFFSET).ok()?,
        line_state: memory.read_i32(manager + RADIO_LINE_STATE_OFFSET).ok()?,
        line_time_s: f32::from_le_bytes(
            memory
                .read_array::<4>(manager + RADIO_LINE_TIME_OFFSET)
                .ok()?,
        ),
        line_flag: memory.read_u8(manager + RADIO_LINE_FLAG_OFFSET).ok()?,
    })
}

extern "system" fn wait_detour(args: *mut c_void) -> u8 {
    let still_waiting = call_original(&ORIGINAL_WAIT, args);
    let _ = panic::catch_unwind(|| note_yielding_native("_Wait"));
    still_waiting
}

extern "system" fn start_radio_wait_detour(args: *mut c_void) -> u8 {
    let starts = panic::catch_unwind(|| wait_state(args as u64) == Some(0)).unwrap_or(false);
    let still_waiting = call_original(&ORIGINAL_START_RADIO_WAIT, args);
    let keep_waiting =
        panic::catch_unwind(|| radio_wait_polled(starts, still_waiting != 0)).unwrap_or(true);
    // Returning 0 is how a polled wait ends: the script step resumes the handler's thread.
    if keep_waiting { still_waiting } else { 0 }
}

#[expect(unsafe_code, reason = "calls the original wait through the trampoline")]
pub(super) fn call_original(original: &AtomicPtr<c_void>, args: *mut c_void) -> u8 {
    let trampoline = original.load(Ordering::Acquire);
    if trampoline.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original wait, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, PolledNative>(trampoline)(args) }
}

pub(super) fn wait_state(args: u64) -> Option<i32> {
    let build = BUILD.get()?;
    InProcessMemory::new(build)
        .read_i32(args + POLLED_NATIVE_STATE_OFFSET)
        .ok()
}

fn radio_wait_polled(starts: bool, still_waiting: bool) -> bool {
    note_yielding_native("_StartRadio");
    let Some(build) = BUILD.get() else {
        return true;
    };
    if starts && still_waiting && (main_menu_open(build) || pause_menu_open(build)) {
        log_line_started_under_menu(build);
    }
    let Some(here) = here(build) else {
        return true;
    };
    let now_ms = net_clock_ms();
    match here.role {
        FollowRole::Host => {
            let mark = HOST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .radio_wait_polled(starts, still_waiting, now_ms);
            if let Some(mark) = mark {
                send_mark(mark);
            }
            true
        }
        FollowRole::Guest => {
            let call = GuestRadioWaitCall {
                starts,
                still_waiting,
                host_in_scene: net::handle().is_some_and(|net| partner_in_cutscene(&net.view())),
                here_in_scene: in_event_scene(build),
                line_advancing: line_advanced_since_last_poll(build, starts),
                now_ms,
            };
            let release = GUEST
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .radio_wait_polled(call, here.local_phase);
            let Some(release) = release else {
                return true;
            };
            stop_radio(build);
            let holder = slot_holder_name().unwrap_or_else(|| "an unknown handler".to_owned());
            log_line(&format_radio_wait_release(&holder, release));
            false
        }
        FollowRole::Unknown => true,
    }
}

fn send_mark(mark: Message) {
    if let Message::FightRadioWait(ended) = &mark {
        log_line(&format!(
            "radio wait: your wait {} in seq {} ended after {} ms, sent to the guest",
            ended.wait_index, ended.seq, ended.host_waited_ms
        ));
    }
    if let Some(net) = net::handle() {
        net.send_relayed(mark);
    }
}

fn log_line_started_under_menu(build: &VerifiedBuild) {
    let Some(radio) = read_radio_state(build) else {
        return;
    };
    let holder = slot_holder_name().unwrap_or_else(|| "an unknown handler".to_owned());
    log_line(&format_radio_line_under_menu(&holder, &radio));
}

fn line_advanced_since_last_poll(build: &VerifiedBuild, starts: bool) -> bool {
    let now = read_radio_state(build).map(|radio| radio.line_time_s);
    let mut last = LINE_TIME_AT_LAST_GUEST_POLL
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    // A new wait has no earlier poll, and a failed read counts as a stalled line.
    let before = if starts { None } else { *last };
    *last = now;
    before.zip(now).is_some_and(|(before, now)| now > before)
}

pub(crate) fn in_event_scene(build: &VerifiedBuild) -> bool {
    InProcessMemory::new(build)
        .read_u8(build.module_base() + u64::from(EVENT_SCENE_BYTE_RVA))
        .is_ok_and(|scene| scene != 0)
}

#[expect(unsafe_code, reason = "calls the game's own radio stop by address")]
fn stop_radio(build: &VerifiedBuild) {
    let base = build.module_base();
    let stop = (base + u64::from(RADIO_STOP_RVA)) as usize as *const ();
    let manager = (base + u64::from(RADIO_MANAGER_RVA)) as usize as *mut c_void;
    // SAFETY: 0x4AB180 is the radio manager's stop in the verified build (manager in, flag in,
    // nothing out): it stops the playing line and, with the flag set, clears the busy byte. The
    // thunk at 0x0CE660 calls it on the same manager; this runs on the game thread in the step.
    unsafe { core::mem::transmute::<*const (), RadioStop>(stop)(manager, 1) };
}

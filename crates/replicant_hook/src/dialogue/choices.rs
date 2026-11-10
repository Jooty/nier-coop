use core::ffi::c_void;
use core::ptr;
use std::cell::Cell;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::dialogue::ChoiceFollower;
use replicant_game::story_writes::{
    MENU_LIST_PICK_STATE_RVA, SELECTOR_CONFIRMED_INDEX_RVA, TALK_LIST_PICK_STATE_RVA,
    TALK_WINDOW_PICK_TARGET_OFFSET,
};
use replicant_proto::{Message, TalkChoice, TalkChoiceKind};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::tick::own_talk_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, NetThread};

static ORIGINAL_TALK_LIST_PICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_MENU_LIST_PICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CONFIRMED_INDEX: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

static ANSWER_TO_TAKE: Mutex<Option<TalkChoice>> = Mutex::new(None);

thread_local! {
    static PICK_STATE_RUNNING: Cell<Option<TalkChoiceKind>> = const { Cell::new(None) };
    static PICKED_INDEX: Cell<Option<i32>> = const { Cell::new(None) };
    static TAKEN_ANSWER: Cell<Option<TalkChoice>> = const { Cell::new(None) };
}

static HOOKED_BUILD: std::sync::OnceLock<VerifiedBuild> = std::sync::OnceLock::new();

type PickState = unsafe extern "system" fn(*mut c_void, *mut c_void);
type ConfirmedIndex = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "hooks the talk window's two pick states and the list confirm reader by address"
    )]
    // SAFETY: each RVA starts the named function in the verified build (see docs
    // off-hand/story-choices.md) and each replacement has the same signature.
    unsafe {
        hook_address(
            "TalkWindow::PickTalkList",
            (base + u64::from(TALK_LIST_PICK_STATE_RVA)) as usize,
            talk_list_pick_detour as *const () as *mut c_void,
            &ORIGINAL_TALK_LIST_PICK,
        );
        hook_address(
            "TalkWindow::PickMenuList",
            (base + u64::from(MENU_LIST_PICK_STATE_RVA)) as usize,
            menu_list_pick_detour as *const () as *mut c_void,
            &ORIGINAL_MENU_LIST_PICK,
        );
        hook_address(
            "Selector::ConfirmedIndex",
            (base + u64::from(SELECTOR_CONFIRMED_INDEX_RVA)) as usize,
            confirmed_index_detour as *const () as *mut c_void,
            &ORIGINAL_CONFIRMED_INDEX,
        );
    }
}

pub(super) fn receive_partner_choices(net: &NetThread, follower: &mut ChoiceFollower, now_ms: u64) {
    for message in net.take_relayed(|message| matches!(message, Message::TalkChoice(_))) {
        if let Message::TalkChoice(choice) = message {
            log_line(&format!(
                "talk-choice: kind={:?} talk={} line={} index={} target={} received",
                choice.kind, choice.talk_id, choice.line, choice.index, choice.target
            ));
            follower.receive(choice, now_ms);
        }
    }
}

pub(super) fn arm_answer(choice: TalkChoice) {
    *ANSWER_TO_TAKE
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(choice);
}

pub(super) fn disarm_answer() {
    *ANSWER_TO_TAKE
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;
}

pub(super) fn answer_is_armed() -> bool {
    ANSWER_TO_TAKE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
}

fn take_armed_answer(kind: TalkChoiceKind) -> Option<TalkChoice> {
    let mut armed = ANSWER_TO_TAKE
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if armed.is_some_and(|choice| choice.kind == kind) {
        armed.take()
    } else {
        None
    }
}

extern "system" fn talk_list_pick_detour(window: *mut c_void, arg: *mut c_void) {
    run_pick_state(
        window,
        arg,
        TalkChoiceKind::TalkList,
        &ORIGINAL_TALK_LIST_PICK,
    );
}

extern "system" fn menu_list_pick_detour(window: *mut c_void, arg: *mut c_void) {
    run_pick_state(
        window,
        arg,
        TalkChoiceKind::MenuList,
        &ORIGINAL_MENU_LIST_PICK,
    );
}

#[expect(
    unsafe_code,
    reason = "calls the original pick state through the trampoline MinHook built"
)]
fn run_pick_state(
    window: *mut c_void,
    arg: *mut c_void,
    kind: TalkChoiceKind,
    original: &AtomicPtr<c_void>,
) {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    PICK_STATE_RUNNING.with(|state| state.set(Some(kind)));
    PICKED_INDEX.with(|index| index.set(None));
    TAKEN_ANSWER.with(|taken| taken.set(None));
    // SAFETY: the trampoline continues into the original state function, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, PickState>(original)(window, arg) };
    PICK_STATE_RUNNING.with(|state| state.set(None));
    if let Some(choice) = TAKEN_ANSWER.with(Cell::take) {
        log_line(&format!(
            "choice: took answer {} of talk {}",
            choice.index, choice.talk_id
        ));
    }
    if let Some(index) = PICKED_INDEX.with(Cell::take) {
        send_choice(window, kind, index);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original confirm reader through the trampoline MinHook built"
)]
extern "system" fn confirmed_index_detour(selector: *mut c_void, pad: *mut c_void) -> i32 {
    let original = ORIGINAL_CONFIRMED_INDEX.load(Ordering::Acquire);
    if original.is_null() {
        return -1;
    }
    // SAFETY: the trampoline continues into the original reader, which has this signature.
    let index =
        unsafe { core::mem::transmute::<*mut c_void, ConfirmedIndex>(original)(selector, pad) };
    let Some(kind) = PICK_STATE_RUNNING.with(Cell::get) else {
        return index;
    };
    if index >= 0 {
        PICKED_INDEX.with(|picked| picked.set(Some(index)));
        return index;
    }
    // The host's answer stands in for this player's confirm press; it is not sent back.
    if let Some(choice) = take_armed_answer(kind) {
        TAKEN_ANSWER.with(|taken| taken.set(Some(choice)));
        return i32::from(choice.index);
    }
    index
}

fn send_choice(window: *mut c_void, kind: TalkChoiceKind, index: i32) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let Some(net) = net::handle() else {
        return;
    };
    let Some(line) = own_talk_line() else {
        log_line("talk-choice: a pick came with no known talk line, not shared");
        return;
    };
    let memory = InProcessMemory::new(build);
    let target = memory
        .read_i32(window as u64 + TALK_WINDOW_PICK_TARGET_OFFSET)
        .unwrap_or(0)
        .cast_unsigned();
    let Ok(index) = u8::try_from(index) else {
        return;
    };
    let choice = TalkChoice {
        talk_id: line.talk_id,
        line: line.line,
        kind,
        index,
        target,
    };
    if choice.validate().is_err() {
        log_line(&format!(
            "talk-choice: kind={kind:?} talk={} index={index} is outside the wire bounds, not shared",
            line.talk_id
        ));
        return;
    }
    log_line(&format!(
        "talk-choice: kind={kind:?} talk={} line={} index={index} target={target} sent",
        line.talk_id, line.line
    ));
    net.send_relayed(Message::TalkChoice(choice));
}

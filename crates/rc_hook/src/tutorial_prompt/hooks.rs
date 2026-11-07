use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::tutorial_prompt::{LocalOpen, OPEN_TUTORIAL_FUNCTION_RVA, TutorialPrompts};
use rc_proto::{FIRST_TUTORIAL_ID, LAST_TUTORIAL_ID, Message, TutorialPrompt};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::net::{self, NetState};

static ORIGINAL_OPEN_TUTORIAL: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static OPENING_FOR_PARTNER: AtomicBool = AtomicBool::new(false);
pub(super) static PROMPTS: Mutex<TutorialPrompts> = Mutex::new(TutorialPrompts::new());

type OpenTutorial = unsafe extern "system" fn(*mut c_void, i32, i32);

pub(super) fn open_for_partner(dialog: *mut c_void, kind: i32, id: u16) -> bool {
    let original = ORIGINAL_OPEN_TUTORIAL.load(Ordering::Acquire);
    if original.is_null() {
        return false;
    }
    OPENING_FOR_PARTNER.store(true, Ordering::Relaxed);
    #[expect(
        unsafe_code,
        reason = "calls the game's tutorial prompt opener through the trampoline MinHook built"
    )]
    // SAFETY: the trampoline continues into the original opener (dialog object, kind, tutorial
    // id); this runs on the game thread in the script tick.
    unsafe {
        core::mem::transmute::<*mut c_void, OpenTutorial>(original)(dialog, kind, i32::from(id));
    }
    OPENING_FOR_PARTNER.store(false, Ordering::Relaxed);
    true
}

pub fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(unsafe_code, reason = "hooks the tutorial prompt opener by address")]
    // SAFETY: the RVA starts the opener (dialog object, kind, tutorial id) in the verified build,
    // and the replacement has the same signature.
    unsafe {
        hook_address(
            "UiDialog::OpenTutorial",
            (base + u64::from(OPEN_TUTORIAL_FUNCTION_RVA)) as usize,
            open_tutorial_detour as *const () as *mut c_void,
            &ORIGINAL_OPEN_TUTORIAL,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original opener through the trampoline MinHook built and reads the dialog state"
)]
extern "system" fn open_tutorial_detour(dialog: *mut c_void, kind: i32, id: i32) {
    let original = ORIGINAL_OPEN_TUTORIAL.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original opener with this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, OpenTutorial>(original)(dialog, kind, id) };
    if OPENING_FOR_PARTNER.load(Ordering::Relaxed) {
        return;
    }
    // SAFETY: the first argument is the game's dialog object, whose first dword is its state.
    let opened = unsafe { ptr::read_volatile(dialog as *const i32) } != 0;
    let _ = panic::catch_unwind(|| local_opened(id, opened));
}

fn local_opened(id: i32, opened: bool) {
    let Ok(id) = u16::try_from(id) else {
        return;
    };
    if !opened || !(FIRST_TUTORIAL_ID..=LAST_TUTORIAL_ID).contains(&id) {
        return;
    }
    let outcome = PROMPTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .local_opened(id);
    if outcome == LocalOpen::AlreadyShown {
        log_line(&format!(
            "tutorial: local prompt {id} opened again (already shown this session), not sent"
        ));
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    if net.view().state != NetState::Connected {
        log_line(&format!("tutorial: local prompt {id} opened, no partner"));
        return;
    }
    net.send_relayed(Message::TutorialPrompt(TutorialPrompt { id }));
    log_line(&format!(
        "tutorial: local prompt {id} opened, sent to partner"
    ));
}

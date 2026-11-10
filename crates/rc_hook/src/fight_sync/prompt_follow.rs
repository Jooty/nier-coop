use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::marker::UI_DIALOG_STATE_RVA;
use rc_game::prompt_follow::{
    MAX_PROMPT_CLOSE_TRIES_PER_PAGE, PromptDismissal, PromptFollow, PromptPageId, PromptView,
};
use rc_game::stay_together::FollowRole;
use rc_game::tutorial_prompt::OPEN_TUTORIAL_FUNCTION_RVA;
use rc_proto::{FightPromptCount, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{BUILD, Here, covers_phase};
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::stay_together::{close_script_prompt_page, pause_menu_open};

static PROMPTS: Mutex<PromptFollow> = Mutex::new(PromptFollow::new());
static LAST_PHASE: Mutex<u16> = Mutex::new(0);
static OPENS_NOT_TAKEN_YET: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());
static ORIGINAL_OPEN_PROMPT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type OpenPrompt = unsafe extern "system" fn(*mut c_void, i32, i32);

pub(super) fn install(build: VerifiedBuild) {
    #[expect(unsafe_code, reason = "hooks the script prompt opener by address")]
    // SAFETY: the RVA starts UiDialog::OpenTutorial in the verified build, void
    // (dialog object, kind, id); docs symbols.tsv and the prologue (rcx dialog, edx kind, r8d id)
    // confirm it, and the replacement has the same signature.
    unsafe {
        hook_address(
            "UiDialog::OpenTutorial",
            (build.module_base() + u64::from(OPEN_TUTORIAL_FUNCTION_RVA)) as usize,
            open_prompt_detour as *const () as *mut c_void,
            &ORIGINAL_OPEN_PROMPT,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original opener through the trampoline MinHook built"
)]
extern "system" fn open_prompt_detour(dialog: *mut c_void, kind: i32, id: i32) {
    let original = ORIGINAL_OPEN_PROMPT.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original opener, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, OpenPrompt>(original)(dialog, kind, id) };
    let _ = panic::catch_unwind(|| note_open_if_it_opened(kind, id));
}

fn note_open_if_it_opened(kind: i32, id: i32) {
    let Some(build) = BUILD.get() else {
        return;
    };
    let state = InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .unwrap_or(0);
    if state == 0 {
        return;
    }
    OPENS_NOT_TAKEN_YET
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((kind, id));
}

pub(super) fn tick(build: &VerifiedBuild, here: Option<Here>, counts: &[FightPromptCount]) {
    let local_phase = here.and_then(|here| here.local_phase);
    let role = here.map(|here| here.role);
    let stream_active = role.is_some() && local_phase.is_some_and(covers_phase);
    if let Some(phase) = local_phase.filter(|_| stream_active) {
        *LAST_PHASE.lock().unwrap_or_else(PoisonError::into_inner) = phase;
    }
    let dialog_state = InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .unwrap_or(0);
    let opens = std::mem::take(
        &mut *OPENS_NOT_TAKEN_YET
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    let mut prompts = PROMPTS.lock().unwrap_or_else(PoisonError::into_inner);
    for (kind, id) in opens {
        prompts.note_open(kind, id);
        log_line(&format!("prompt follow: script prompt {kind}/{id} opened"));
    }
    if role == Some(FollowRole::Guest) {
        for count in counts
            .iter()
            .filter(|count| Some(count.phase) == local_phase)
        {
            prompts.receive_host_dismissal(PromptDismissal {
                page: PromptPageId {
                    kind: count.kind,
                    id: count.id,
                    page: count.page,
                },
                times: count.dismissed,
            });
            log_line(&format!(
                "prompt follow: the host dismissed {}/{} page {} ({} times)",
                count.kind, count.id, count.page, count.dismissed
            ));
        }
    }
    let step = prompts.tick(PromptView {
        stream_active,
        dialog_state,
        pause_menu_open: pause_menu_open(build),
        now_ms: net_clock_ms(),
    });
    drop(prompts);
    if role == Some(FollowRole::Host)
        && let Some(dismissal) = step.dismissed
        && let Some(net) = net::handle()
    {
        let phase = *LAST_PHASE.lock().unwrap_or_else(PoisonError::into_inner);
        net.send_relayed(Message::FightPromptCount(FightPromptCount {
            phase,
            kind: dismissal.page.kind,
            id: dismissal.page.id,
            page: dismissal.page.page,
            dismissed: dismissal.times,
        }));
    }
    if role != Some(FollowRole::Guest) {
        return;
    }
    if let Some(close) = step.close {
        log_line(&format!(
            "prompt follow: closing your {}/{} page {}, the host dismissed it",
            close.page.kind, close.page.id, close.page.page
        ));
        close_script_prompt_page(build);
    }
    if let Some(page) = step.gave_up_on {
        log_line(&format!(
            "prompt follow: closing your {}/{} page {} did not work after {MAX_PROMPT_CLOSE_TRIES_PER_PAGE} tries, left alone",
            page.kind, page.id, page.page
        ));
    }
}

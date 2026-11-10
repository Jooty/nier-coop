use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::marker::UI_DIALOG_STATE_RVA;
use rc_game::prompt_follow::{MAX_PROMPT_CLOSE_TRIES_PER_PAGE, PromptFollow, PromptView};
use rc_game::stay_together::FollowRole;
use rc_proto::{FightPromptCount, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::state::{Here, covers_phase};
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::stay_together::{close_script_prompt_page, pause_menu_open};

static PROMPTS: Mutex<PromptFollow> = Mutex::new(PromptFollow::new());
static LAST_PHASE: Mutex<u16> = Mutex::new(0);

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
    let mut prompts = PROMPTS.lock().unwrap_or_else(PoisonError::into_inner);
    if role == Some(FollowRole::Guest) {
        for count in counts
            .iter()
            .filter(|count| Some(count.phase) == local_phase)
        {
            prompts.receive_host_count(count.dismissed);
            log_line(&format!(
                "prompt follow: the host dismissed {}, you dismissed {}",
                count.dismissed,
                prompts.dismissed()
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
        && let Some(dismissed) = step.send_count
        && let Some(net) = net::handle()
    {
        let phase = *LAST_PHASE.lock().unwrap_or_else(PoisonError::into_inner);
        net.send_relayed(Message::FightPromptCount(FightPromptCount {
            phase,
            dismissed,
        }));
    }
    if role != Some(FollowRole::Guest) {
        return;
    }
    if let Some(close) = step.close {
        log_line(&format!(
            "prompt follow: closing your page {}, the host dismissed {}",
            close.page, close.host_dismissed
        ));
        close_script_prompt_page(build);
    }
    if let Some(page) = step.gave_up_on_page {
        log_line(&format!(
            "prompt follow: closing your page {page} did not work after {MAX_PROMPT_CLOSE_TRIES_PER_PAGE} tries, left alone"
        ));
    }
}

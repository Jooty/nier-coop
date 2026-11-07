use std::sync::PoisonError;

use rc_game::GameMemory;
use rc_game::fade_sync::{FADE_ACTIVE_OFFSET, FADE_LAYER_RVA};
use rc_game::marker::UI_DIALOG_STATE_RVA;
use rc_game::tutorial_prompt::{PromptView, TUTORIAL_QUEUE_PROMPT_KIND, may_open_now};
use rc_game::ui_scenes::read_open_ui_scenes;
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::local_in_cutscene;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;
use crate::tutorial_prompt::hooks::{PROMPTS, open_for_partner};

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let mut prompts = PROMPTS.lock().unwrap_or_else(PoisonError::into_inner);
    for message in net.take_relayed(|message| matches!(message, Message::TutorialPrompt(_))) {
        let Message::TutorialPrompt(prompt) = message else {
            continue;
        };
        if prompts.partner_opened(prompt.id) {
            log_line(&format!(
                "tutorial: partner opened prompt {}, waiting to show it here",
                prompt.id
            ));
        } else {
            log_line(&format!(
                "tutorial: partner prompt {} ignored (already shown or waiting)",
                prompt.id
            ));
        }
    }
    if prompts.waiting_count() == 0 {
        return;
    }
    let memory = InProcessMemory::new(build);
    let base = memory.module_base();
    let ui_dialog_state = memory
        .read_i32(base + u64::from(UI_DIALOG_STATE_RVA))
        .unwrap_or(1);
    let fade_active = !memory
        .read_u8(base + u64::from(FADE_LAYER_RVA) + FADE_ACTIVE_OFFSET)
        .is_ok_and(|active| active == 0);
    let pause_menu_open =
        read_open_ui_scenes(&memory).map_or(true, |scenes| scenes.pause_menu_open());
    let view = PromptView {
        in_cutscene: local_in_cutscene(),
        fade_active,
        ui_dialog_state,
        pause_menu_open,
    };
    let Some(id) = prompts.next_to_open(may_open_now(view)) else {
        return;
    };
    let dialog = (base + u64::from(UI_DIALOG_STATE_RVA)) as *mut core::ffi::c_void;
    if open_for_partner(dialog, TUTORIAL_QUEUE_PROMPT_KIND, id) {
        log_line(&format!("tutorial: opened the partner's prompt {id}"));
    }
}

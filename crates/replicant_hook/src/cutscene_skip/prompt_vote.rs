use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::callback_gate::GateRole;
use replicant_game::cutscene_skip::{
    PopVerdict, PromptAnswer, SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS, SkipPrompt, SkipPromptVote,
    VoteEvent, VoteStep, VoteView, skip_prompt, skip_prompt_answer,
};
use replicant_game::marker::UI_DIALOG_STATE_RVA;
use replicant_proto::{Message, SkipPromptAnswer, SkipPromptVerdict, StoryCall};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::callback_gate::{gate_role, shared_phase};
use crate::cutscene_skip::state::{is_connected, now_ms, partner_name};
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::fight_sync::{open_ui_dialog_page, shown_ui_dialog_id};
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetThread};
use crate::stay_together::{close_script_prompt_page, pause_menu_open};
use crate::story_relay::inject::queue_call_unmarked;

static VOTE: Mutex<SkipPromptVote> = Mutex::new(SkipPromptVote::new());

pub(crate) fn hold_skip_prompt_answer(
    build: &VerifiedBuild,
    name: &str,
    queued_by_the_mod: bool,
) -> bool {
    let Some((prompt, answer)) = skip_prompt_answer(name) else {
        return false;
    };
    let Some(net) = net::handle() else {
        return false;
    };
    let view = net.view();
    let mut vote = VOTE.lock().unwrap_or_else(PoisonError::into_inner);
    if queued_by_the_mod {
        if gate_role(&view) == Some(GateRole::Guest) {
            let step = vote.host_answer_ran(name);
            drop(vote);
            apply(build, net, &view, prompt, step);
        }
        return false;
    }
    let vote_view = vote_view(build, &view, prompt);
    let (verdict, step) = vote.local_pop(name, vote_view);
    drop(vote);
    if verdict == PopVerdict::Drop {
        log_line(&format!(
            "skip prompt: your game's {name} ({}) on prompt {} is held for the vote",
            answer_label(answer),
            prompt.prompt
        ));
    }
    apply(build, net, &view, prompt, step);
    verdict == PopVerdict::Drop
}

pub(super) fn tick(build: &VerifiedBuild, net: &NetThread, view: &NetShared) {
    let messages = net.take_relayed(|message| {
        matches!(
            message,
            Message::SkipPromptAnswer(_) | Message::SkipPromptVerdict(_)
        )
    });
    for message in messages {
        let (phase, prompt_id, skip, verdict) = match message {
            Message::SkipPromptAnswer(answer) => (answer.phase, answer.prompt, answer.skip, false),
            Message::SkipPromptVerdict(verdict) => {
                (verdict.phase, verdict.prompt, verdict.skip, true)
            }
            _ => continue,
        };
        if Some(phase) != shared_phase(view) {
            continue;
        }
        let Some(prompt) = skip_prompt(prompt_id) else {
            continue;
        };
        let mut vote = VOTE.lock().unwrap_or_else(PoisonError::into_inner);
        let step = if verdict {
            if gate_role(view) != Some(GateRole::Guest) {
                continue;
            }
            vote.host_verdict(prompt_id, skip)
        } else {
            vote.partner_answer(prompt_id, skip, vote_view(build, view, prompt))
        };
        drop(vote);
        apply(build, net, view, prompt, step);
    }
    let mut vote = VOTE.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(prompt) = vote.open_prompt() else {
        return;
    };
    let vote_view = vote_view(build, view, prompt);
    let step = vote.tick(vote_view);
    drop(vote);
    apply(build, net, view, prompt, step);
}

fn vote_view(build: &VerifiedBuild, view: &NetShared, prompt: SkipPrompt) -> VoteView {
    let role = gate_role(view);
    let pause_menu = pause_menu_open(build);
    let dialog_state = InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .unwrap_or(0);
    VoteView {
        host: role == Some(GateRole::Host),
        partner_present: is_connected(view) && role.is_some() && shared_phase(view).is_some(),
        page_shown_here: shown_ui_dialog_id(build) == Some(i32::from(prompt.prompt)),
        dialog_free: dialog_state == 0 && !pause_menu,
        now_ms: now_ms(),
    }
}

fn apply(
    build: &VerifiedBuild,
    net: &NetThread,
    view: &NetShared,
    prompt: SkipPrompt,
    step: VoteStep,
) {
    let partner = partner_name(view);
    let host = gate_role(view) == Some(GateRole::Host);
    if let Some(event) = step.event {
        announce(event, prompt, host, &partner, step.fire);
    }
    if host
        && let Some(verdict) = step.verdict
        && let Some(phase) = shared_phase(view)
    {
        net.send_relayed(Message::SkipPromptVerdict(SkipPromptVerdict {
            phase,
            prompt: verdict.prompt,
            skip: verdict.skip,
        }));
    }
    if step.close_page && shown_ui_dialog_id(build) == Some(i32::from(prompt.prompt)) {
        log_line(&format!(
            "skip prompt: closing prompt {} here",
            prompt.prompt
        ));
        close_script_prompt_page(build);
    }
    if let Some(sent) = step.send
        && let Some(phase) = shared_phase(view)
    {
        net.send_relayed(Message::SkipPromptAnswer(SkipPromptAnswer {
            phase,
            prompt: sent.prompt,
            skip: sent.skip,
        }));
    }
    if let Some(page) = step.open_page {
        let opened = open_ui_dialog_page(build, i32::from(page));
        log_line(&format!(
            "skip prompt: showing prompt {page} here for {partner}'s ask (opened {opened})"
        ));
    }
    if let Some(callback) = step.fire {
        let phase = view.local.as_ref().map_or(0, |local| local.phase);
        let call = StoryCall {
            phase,
            pack: 0,
            function: callback.to_owned(),
            args: Vec::new(),
        };
        let queued = queue_call_unmarked(build, &call).is_some();
        log_line(&format!(
            "skip prompt: {callback} queued here (queued {queued})"
        ));
    }
}

fn announce(
    event: VoteEvent,
    prompt: SkipPrompt,
    host: bool,
    partner: &str,
    fire: Option<&'static str>,
) {
    let id = prompt.prompt;
    let fired = fire.unwrap_or("nothing");
    let skip = prompt.skip_answer;
    let how_to_agree = if prompt.shows_page {
        "answer the prompt to agree"
    } else {
        "press skip to agree"
    };
    let (line, toast) = match (event, host) {
        (VoteEvent::AskedAndHeld, true) => (
            format!("you answered skip on prompt {id}, held until {partner} answers their own"),
            Some(format!("You want to skip; waiting for {partner}")),
        ),
        (VoteEvent::AskedAndHeld, false) => (
            format!("you answered skip on prompt {id}, held and sent to the host, who decides"),
            Some(format!("You want to skip; waiting for {partner}")),
        ),
        (VoteEvent::PartnerAsks, _) => (
            format!("{partner} answered skip on prompt {id}, nobody answers for you"),
            Some(format!("{partner} wants to skip; {how_to_agree}")),
        ),
        (VoteEvent::SkipAgreed, _) => (
            format!("both players agreed to skip prompt {id}, {skip} fires here"),
            Some("Both agreed; skipping".to_owned()),
        ),
        (VoteEvent::SkipAlone, _) => (
            format!(
                "prompt {id} skipped with no partner in the same place, {skip} runs as the game chose"
            ),
            None,
        ),
        (VoteEvent::DeclinedHere, _) => (
            format!("you answered no on prompt {id}, it runs here and is sent to {partner}"),
            None,
        ),
        (VoteEvent::DeclinedByPartner, true) => (
            format!("{partner} answered no on prompt {id}, the skip does not fire ({fired} fires)"),
            Some(format!("{partner} does not want to skip")),
        ),
        (VoteEvent::DeclinedByPartner, false) => (
            format!("{partner} answered no on prompt {id}, the vote is over"),
            Some(format!("{partner} does not want to skip")),
        ),
        (VoteEvent::GaveUp, true) => (
            format!(
                "no agreement on prompt {id} within {SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS} ms, the skip does not fire ({fired} fires)"
            ),
            None,
        ),
        (VoteEvent::GaveUp, false) => (
            format!(
                "the vote on prompt {id} ended without the host's verdict ({fired} fires here)"
            ),
            None,
        ),
        (VoteEvent::HostAnswerRan, _) => (
            format!("the host's answer to prompt {id} ran here, the vote is over"),
            None,
        ),
        (VoteEvent::HostChoseHere, _) => (
            format!("the host's verdict on prompt {id} came, {fired} fires here too"),
            None,
        ),
        (VoteEvent::SequenceEnded, _) => (
            format!("the sequence of prompt {id} ended on its own, the vote is over"),
            None,
        ),
    };
    log_line(&format!("skip prompt: {line}"));
    if let Some(toast) = toast {
        post_event(&toast);
    }
}

fn answer_label(answer: PromptAnswer) -> &'static str {
    match answer {
        PromptAnswer::Skip => "skip",
        PromptAnswer::Other => "no",
    }
}

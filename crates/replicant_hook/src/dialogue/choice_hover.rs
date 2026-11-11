use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};

use replicant_game::GameMemory;
use replicant_game::dialogue::{
    ChoiceHoverTracker, FollowedPromptFrame, OwnChoiceMarker, PartnerChoiceHover,
    TALK_UPDATE_INPUT_BYTES, followed_prompt_frame, marker_move, read_choice_prompt,
    write_highlighted_option,
};
use replicant_game::marker::UI_DIALOG_STATE_RVA;
use replicant_proto::{ChoiceHover, DialogueLine, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::hooks::FOLLOWING_HOST_TALK;
use crate::dialogue::tick::own_talk_line;
use crate::memory::InProcessMemory;
use crate::net::{NetThread, net_clock_ms};

const CHOICE_TAGS_NOT_DRAWN_WHEN_OLDER_THAN_MS: u64 = 500;

static PARTNER_HOVER: Mutex<PartnerChoiceHover> = Mutex::new(PartnerChoiceHover::new());
static OWN_MARKER: Mutex<OwnChoiceMarker> = Mutex::new(OwnChoiceMarker::new());
static HOVER_TRACKER: Mutex<ChoiceHoverTracker> = Mutex::new(ChoiceHoverTracker::new());
static SHOWN_TAGS: Mutex<Option<PublishedTags>> = Mutex::new(None);

/// The option numbers the overlay names beside an open answer prompt, counted from 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChoiceTags {
    pub(crate) partner_option: Option<u8>,
    pub(crate) own_marker: Option<u8>,
}

#[derive(Clone, Copy)]
struct PublishedTags {
    tags: ChoiceTags,
    at_ms: u64,
}

pub(super) fn receive_partner_hovers(net: &NetThread) {
    for message in net.take_relayed(|message| matches!(message, Message::ChoiceHover(_))) {
        if let Message::ChoiceHover(hover) = message {
            log_line(&format!(
                "choice: partner is on option {} of talk {} line {}",
                hover.index, hover.talk_id, hover.line
            ));
            PARTNER_HOVER
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .receive(hover);
        }
    }
}

pub(super) fn partner_moved_to(partner_line: Option<DialogueLine>) {
    PARTNER_HOVER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .partner_moved_to(partner_line);
}

pub(super) fn forget_hovers() {
    PARTNER_HOVER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    OWN_MARKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    *SHOWN_TAGS.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

/// Sends the option this player is on and publishes what the overlay names. Every tick.
pub(super) fn share_own_hover(
    net: &NetThread,
    memory: &InProcessMemory,
    own_talk: Option<DialogueLine>,
    partner_in_play: bool,
) {
    let prompt = read_choice_prompt(memory).filter(|_| own_talk.is_some());
    let line = own_talk.filter(|_| prompt.is_some());
    let option_count = prompt.map_or(0, |prompt| prompt.option_count);
    let follows_host = FOLLOWING_HOST_TALK.load(Ordering::Acquire);
    let partner_option = {
        let mut partner_hover = PARTNER_HOVER.lock().unwrap_or_else(PoisonError::into_inner);
        if !partner_in_play {
            partner_hover.forget();
        }
        partner_hover.on_prompt(line, option_count)
    };
    let own_marker = OWN_MARKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(
            line.filter(|_| follows_host),
            option_count,
            marker_start(partner_option, prompt.and_then(|prompt| prompt.highlighted)),
            0,
        );
    let own_index = if follows_host {
        own_marker
    } else {
        prompt
            .filter(|prompt| prompt.on_list)
            .and_then(|prompt| prompt.highlighted)
    };
    let shown = line
        .zip(own_index)
        .map(|(line, index)| ChoiceHover {
            talk_id: line.talk_id,
            line: line.line,
            index,
        })
        .filter(|hover| hover.validate().is_ok());
    let to_send = HOVER_TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(shown, partner_in_play);
    if let Some(hover) = to_send {
        log_line(&format!(
            "choice: you are on option {} of talk {} line {}{}",
            hover.index,
            hover.talk_id,
            hover.line,
            if follows_host {
                " (a marker, the host decides)"
            } else {
                ""
            }
        ));
        net.send_relayed(Message::ChoiceHover(hover));
    }
    let tags = ChoiceTags {
        // A guest that follows sees the host's option as the game's own highlight.
        partner_option: partner_option.filter(|_| !follows_host),
        own_marker,
    };
    let shown_tags = line
        .filter(|_| !menu_covers_the_talk_window(memory))
        .map(|_| PublishedTags {
            tags,
            at_ms: net_clock_ms(),
        });
    *SHOWN_TAGS.lock().unwrap_or_else(PoisonError::into_inner) = shown_tags;
}

/// Runs before the talk window's update, on the game thread: moves the guest's marker with the
/// guest's own buttons and puts the host's highlight on the game's cursor.
pub(super) fn before_window_update(
    build: &VerifiedBuild,
    update_input: &[u8; TALK_UPDATE_INPUT_BYTES],
    follows_host: bool,
) -> FollowedPromptFrame {
    let mut memory = InProcessMemory::new(build);
    // The window's own update does nothing under this player's menu, and the menu has the buttons.
    let prompt = read_choice_prompt(&memory).filter(|_| !menu_covers_the_talk_window(&memory));
    if prompt.is_none() {
        return followed_prompt_frame(follows_host, None, None, false);
    }
    let line = own_talk_line();
    let option_count = prompt.map_or(0, |prompt| prompt.option_count);
    let host_hover = PARTNER_HOVER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .on_prompt(line, option_count);
    if follows_host {
        let moved = prompt.map_or(0, |prompt| marker_move(prompt.axis, update_input));
        OWN_MARKER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .step(
                line,
                option_count,
                marker_start(host_hover, prompt.and_then(|prompt| prompt.highlighted)),
                moved,
            );
    }
    let frame = followed_prompt_frame(
        follows_host,
        prompt,
        host_hover,
        crate::dialogue::choices::answer_is_armed(),
    );
    if let Some(index) = frame.highlight
        && write_highlighted_option(&mut memory, index).is_err()
    {
        log_line("choice: the host's highlighted option could not be written, the cursor stays");
    }
    frame
}

pub(super) fn menu_covers_the_talk_window(memory: &InProcessMemory) -> bool {
    memory
        .read_i32(memory.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .is_ok_and(|state| state != 0)
}

pub(crate) fn shown_choice_tags() -> Option<ChoiceTags> {
    let published = (*SHOWN_TAGS.lock().unwrap_or_else(PoisonError::into_inner))?;
    let fresh =
        net_clock_ms().saturating_sub(published.at_ms) <= CHOICE_TAGS_NOT_DRAWN_WHEN_OLDER_THAN_MS;
    fresh.then_some(published.tags)
}

fn marker_start(partner_option: Option<u8>, highlighted: Option<u8>) -> u8 {
    partner_option.or(highlighted).unwrap_or(0)
}

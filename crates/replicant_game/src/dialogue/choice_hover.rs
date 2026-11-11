use replicant_proto::{
    ChoiceHover, DialogueLine, MAX_TALK_CHOICE_INDEX, TalkChoice, TalkChoiceKind,
};

use crate::dialogue::{
    CHOICE_SELECTOR_AXIS_HORIZONTAL, CHOICE_SELECTOR_AXIS_OFFSET, CHOICE_SELECTOR_AXIS_VERTICAL,
    CHOICE_SELECTOR_CANCEL_INDEX_OFFSET, CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET,
    CHOICE_SELECTOR_OPTION_COUNT_OFFSET, TALK_PLAYER_CHOICE_SELECTOR_OFFSET,
    TALK_PLAYER_PROMPT_KIND_OFFSET, TALK_PLAYER_TALK_LIST_OPTION_COUNT_OFFSET,
    TALK_STATE_PROMPT_CURSOR, TALK_UPDATE_INPUT_BUTTON_PRESSED_BIT,
    TALK_UPDATE_INPUT_ENABLED_OFFSET, TALK_UPDATE_INPUT_HORIZONTAL_NEXT_OFFSETS,
    TALK_UPDATE_INPUT_HORIZONTAL_PREVIOUS_OFFSETS, TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS,
    TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS, TALK_WINDOW_PLAYER_OFFSET, TALK_WINDOW_RVA,
    open_prompt_kind,
};
use crate::memory::{GameMemory, MemoryError};

/// An answer prompt open in this game's own talk window. `option_count` is 0 when the list could
/// not be read; then nothing is highlighted or marked, and the answer itself still works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoicePrompt {
    pub kind: TalkChoiceKind,
    pub on_list: bool,
    pub option_count: u8,
    pub highlighted: Option<u8>,
    pub axis: i32,
}

/// What the guest's copy of a prompt the host leads does this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FollowedPromptFrame {
    pub buttons_off: bool,
    pub enter_list: bool,
    pub highlight: Option<u8>,
}

/// Whose index the prompt's confirm reader hands to the game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptPick {
    Nothing,
    Own(i32),
    Host(u8),
}

/// Sends this player's highlighted option when it changes, and again when a partner joins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChoiceHoverTracker {
    sent: Option<ChoiceHover>,
    partner_was_in_play: bool,
}

/// The partner's latest highlighted option, shown only on the same line of the same talk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartnerChoiceHover {
    hover: Option<ChoiceHover>,
}

/// The guest's own highlight at a prompt the host decides: it moves a marker, never the cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnChoiceMarker {
    at: Option<(DialogueLine, u8)>,
}

impl ChoiceHoverTracker {
    pub const fn new() -> Self {
        Self {
            sent: None,
            partner_was_in_play: false,
        }
    }

    pub fn observe(
        &mut self,
        shown: Option<ChoiceHover>,
        partner_in_play: bool,
    ) -> Option<ChoiceHover> {
        let partner_joined = partner_in_play && !self.partner_was_in_play;
        self.partner_was_in_play = partner_in_play;
        let changed = shown != self.sent;
        self.sent = shown;
        shown.filter(|_| partner_in_play && (changed || partner_joined))
    }
}

impl PartnerChoiceHover {
    pub const fn new() -> Self {
        Self { hover: None }
    }

    pub fn receive(&mut self, hover: ChoiceHover) {
        self.hover = Some(hover);
    }

    pub fn forget(&mut self) {
        self.hover = None;
    }

    /// A hover belongs to the line the partner was on; it goes when the partner leaves that line.
    pub fn partner_moved_to(&mut self, partner_line: Option<DialogueLine>) {
        let still_there = self
            .hover
            .zip(partner_line)
            .is_some_and(|(hover, line)| hover.talk_id == line.talk_id && hover.line == line.line);
        if !still_there {
            self.hover = None;
        }
    }

    pub fn on_prompt(&self, own_line: Option<DialogueLine>, option_count: u8) -> Option<u8> {
        let own = own_line?;
        self.hover
            .filter(|hover| hover.talk_id == own.talk_id && hover.line == own.line)
            .map(|hover| hover.index)
            .filter(|index| *index < option_count)
    }
}

impl OwnChoiceMarker {
    pub const fn new() -> Self {
        Self { at: None }
    }

    /// `prompt_line` is the line of the prompt the host leads, or None when there is none.
    pub fn step(
        &mut self,
        prompt_line: Option<DialogueLine>,
        option_count: u8,
        starts_at: u8,
        moved: i8,
    ) -> Option<u8> {
        let Some(line) = prompt_line.filter(|_| option_count > 0) else {
            self.at = None;
            return None;
        };
        let last = option_count - 1;
        let index = match self.at {
            Some((at_line, index)) if at_line == line => index.min(last),
            _ => starts_at.min(last),
        };
        let count = i16::from(option_count);
        let index = (i16::from(index) + i16::from(moved)).rem_euclid(count);
        let index = u8::try_from(index).unwrap_or(0);
        self.at = Some((line, index));
        Some(index)
    }

    pub fn forget(&mut self) {
        self.at = None;
    }
}

pub fn read_choice_prompt<M: GameMemory>(memory: &M) -> Option<ChoicePrompt> {
    let window = memory.module_base() + u64::from(TALK_WINDOW_RVA);
    if memory.read_i32(window).ok()? == 0 {
        return None;
    }
    let player = window + TALK_WINDOW_PLAYER_OFFSET;
    let state = memory.read_i32(player).ok()?;
    let kind_field = memory
        .read_i32(player + TALK_PLAYER_PROMPT_KIND_OFFSET)
        .unwrap_or(0);
    let kind = open_prompt_kind(state, kind_field)?;
    let selector = player + TALK_PLAYER_CHOICE_SELECTOR_OFFSET;
    let option_count = memory
        .read_i32(selector + CHOICE_SELECTOR_OPTION_COUNT_OFFSET)
        .ok()
        .and_then(|count| u8::try_from(count).ok())
        .filter(|count| *count <= MAX_TALK_CHOICE_INDEX + 1)
        .unwrap_or(0);
    let highlighted = memory
        .read_i32(selector + CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET)
        .ok()
        .and_then(|index| u8::try_from(index).ok())
        .filter(|index| *index < option_count);
    let axis = memory
        .read_i32(selector + CHOICE_SELECTOR_AXIS_OFFSET)
        .unwrap_or(-1);
    Some(ChoicePrompt {
        kind,
        on_list: state != TALK_STATE_PROMPT_CURSOR,
        option_count,
        highlighted,
        axis,
    })
}

/// The game redraws the highlight itself on its next frame, as it does after a key press.
pub fn write_highlighted_option<M: GameMemory>(
    memory: &mut M,
    index: u8,
) -> Result<(), MemoryError> {
    let selector = memory.module_base()
        + u64::from(TALK_WINDOW_RVA)
        + TALK_WINDOW_PLAYER_OFFSET
        + TALK_PLAYER_CHOICE_SELECTOR_OFFSET;
    memory.write_i32(
        selector + CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET,
        i32::from(index),
    )
}

/// Whether the host's answer names an option this game's open prompt has: one of the listed
/// options, or the index the prompt gives a cancel press.
pub fn host_answer_fits<M: GameMemory>(memory: &M, choice: &TalkChoice) -> bool {
    if choice.kind == TalkChoiceKind::MenuList {
        return true;
    }
    let player = memory.module_base() + u64::from(TALK_WINDOW_RVA) + TALK_WINDOW_PLAYER_OFFSET;
    let index = i32::from(choice.index);
    let listed = memory
        .read_i32(player + TALK_PLAYER_TALK_LIST_OPTION_COUNT_OFFSET)
        .is_ok_and(|count| index < count);
    let cancel = memory
        .read_i32(player + TALK_PLAYER_CHOICE_SELECTOR_OFFSET + CHOICE_SELECTOR_CANCEL_INDEX_OFFSET)
        .is_ok_and(|cancel_index| index == cancel_index);
    listed || cancel
}

/// `host_hover` is the host's highlighted option on this same prompt, if it is known.
pub fn followed_prompt_frame(
    follows_host: bool,
    prompt: Option<ChoicePrompt>,
    host_hover: Option<u8>,
    answer_armed: bool,
) -> FollowedPromptFrame {
    let Some(prompt) = prompt else {
        return FollowedPromptFrame {
            buttons_off: follows_host,
            ..FollowedPromptFrame::default()
        };
    };
    let host_on_list = follows_host && host_hover.is_some();
    FollowedPromptFrame {
        buttons_off: follows_host,
        enter_list: !prompt.on_list && (answer_armed || host_on_list),
        highlight: host_hover.filter(|hover| {
            follows_host
                && prompt.on_list
                && *hover < prompt.option_count
                && prompt.highlighted != Some(*hover)
        }),
    }
}

/// A guest that follows the host never picks or cancels by its own press; the host's answer
/// stands in for the press on every copy that has it.
pub fn prompt_pick(own_index: i32, follows_host: bool, host_answer: Option<u8>) -> PromptPick {
    if own_index >= 0 && !follows_host {
        return PromptPick::Own(own_index);
    }
    host_answer.map_or(PromptPick::Nothing, PromptPick::Host)
}

/// One step back, one step on or none, from the buttons pressed this frame along the list's axis.
pub fn marker_move(axis: i32, update_input: &[u8]) -> i8 {
    let (previous, next) = match axis {
        CHOICE_SELECTOR_AXIS_VERTICAL => (
            TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS,
            TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS,
        ),
        CHOICE_SELECTOR_AXIS_HORIZONTAL => (
            TALK_UPDATE_INPUT_HORIZONTAL_PREVIOUS_OFFSETS,
            TALK_UPDATE_INPUT_HORIZONTAL_NEXT_OFFSETS,
        ),
        _ => return 0,
    };
    let enabled = update_input
        .get(TALK_UPDATE_INPUT_ENABLED_OFFSET)
        .is_some_and(|flag| *flag != 0);
    if !enabled {
        return 0;
    }
    let pressed = |offsets: [usize; 4]| {
        offsets.iter().any(|offset| {
            update_input
                .get(*offset)
                .is_some_and(|button| button & TALK_UPDATE_INPUT_BUTTON_PRESSED_BIT != 0)
        })
    };
    i8::from(pressed(next)) - i8::from(pressed(previous))
}

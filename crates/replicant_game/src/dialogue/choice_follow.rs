use replicant_proto::{DialogueLine, TalkChoice, TalkChoiceKind};

use crate::dialogue::{
    CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS, TALK_PROMPT_KIND_MENU_LIST,
    TALK_PROMPT_KIND_TALK_LIST, TALK_STATE_PROMPT_CURSOR, TALK_STATE_PROMPT_MENU_LIST,
    TALK_STATE_PROMPT_TALK_LIST,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChoiceStep {
    Take(TalkChoice),
    GiveUp(TalkChoice),
    Drop(TalkChoice),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChoiceFollowInput {
    pub follows_host: bool,
    pub own_talk: Option<DialogueLine>,
    pub prompt: Option<TalkChoiceKind>,
    pub now_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingChoice {
    choice: TalkChoice,
    received_ms: u64,
    seen_own_talk: bool,
    taken: bool,
}

/// The host's answer to a prompt, kept until the guest's own copy of the talk can take it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChoiceFollower {
    pending: Option<PendingChoice>,
}

/// The clock the wait for the host's answer runs on. It stands still while this player's menu
/// covers the talk window, because the window cannot take an answer under a menu.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FollowClock {
    last_ms: Option<u64>,
    stood_still_ms: u64,
}

impl FollowClock {
    pub const fn new() -> Self {
        Self {
            last_ms: None,
            stood_still_ms: 0,
        }
    }

    pub fn now(&mut self, now_ms: u64, menu_open: bool) -> u64 {
        if menu_open && let Some(last_ms) = self.last_ms {
            self.stood_still_ms += now_ms.saturating_sub(last_ms);
        }
        self.last_ms = Some(now_ms);
        now_ms.saturating_sub(self.stood_still_ms)
    }
}

impl ChoiceFollower {
    pub const fn new() -> Self {
        Self { pending: None }
    }

    pub fn receive(&mut self, choice: TalkChoice, now_ms: u64) {
        self.pending = Some(PendingChoice {
            choice,
            received_ms: now_ms,
            seen_own_talk: false,
            taken: false,
        });
    }

    pub fn forget(&mut self) {
        self.pending = None;
    }

    pub fn is_waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn step(&mut self, input: ChoiceFollowInput) -> Option<ChoiceStep> {
        let pending = self.pending.as_mut()?;
        let choice = pending.choice;
        if !input.follows_host {
            self.pending = None;
            return Some(ChoiceStep::Drop(choice));
        }
        let on_talk = input
            .own_talk
            .is_some_and(|own| own.talk_id == choice.talk_id);
        if on_talk {
            pending.seen_own_talk = true;
        } else if pending.seen_own_talk {
            let taken = pending.taken;
            self.pending = None;
            return (!taken).then_some(ChoiceStep::Drop(choice));
        }
        if pending.taken && input.prompt.is_none() {
            self.pending = None;
            return None;
        }
        let age = input.now_ms.saturating_sub(pending.received_ms);
        if age >= CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS {
            self.pending = None;
            return Some(if on_talk {
                ChoiceStep::GiveUp(choice)
            } else {
                ChoiceStep::Drop(choice)
            });
        }
        let at_the_prompt = on_talk
            && input.prompt == Some(choice.kind)
            && input.own_talk.is_some_and(|own| own.line == choice.line);
        if at_the_prompt && !pending.taken {
            pending.taken = true;
            return Some(ChoiceStep::Take(choice));
        }
        None
    }
}

/// The kind of answer prompt the talk player is showing, from its state and its prompt kind field.
pub fn open_prompt_kind(player_state: i32, prompt_kind_field: i32) -> Option<TalkChoiceKind> {
    match player_state {
        TALK_STATE_PROMPT_TALK_LIST => Some(TalkChoiceKind::TalkList),
        TALK_STATE_PROMPT_MENU_LIST => Some(TalkChoiceKind::MenuList),
        TALK_STATE_PROMPT_CURSOR => match prompt_kind_field {
            TALK_PROMPT_KIND_TALK_LIST => Some(TalkChoiceKind::TalkList),
            TALK_PROMPT_KIND_MENU_LIST => Some(TalkChoiceKind::MenuList),
            _ => None,
        },
        _ => None,
    }
}

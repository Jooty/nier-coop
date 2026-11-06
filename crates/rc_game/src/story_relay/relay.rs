use rc_proto::StoryRefusalReason;

use crate::stay_together::FollowRole;
use crate::story_calls::StoryCallKind;
use crate::story_relay::{CallKey, InjectedMatch, StoryRelayLedger};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireFacts {
    pub enabled: bool,
    pub connected: bool,
    pub role: FollowRole,
    pub local_in_play: bool,
    pub partner_in_same_phase: bool,
    pub kind: Option<StoryCallKind>,
    pub owned_elsewhere: bool,
    pub args_relayable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireAction {
    RunLocally,
    RunInjected(InjectedMatch),
    RunAndAnnounce,
    SuppressAndAsk,
    DropDuplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunFacts {
    pub enabled: bool,
    pub role: FollowRole,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub call_phase: u16,
    pub known_story_call: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropReason {
    Disabled,
    WrongRole,
    NotInPlay,
    WrongPhase,
    NotAStoryCall,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerAction {
    InjectAndBroadcast,
    Drop(DropReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunAction {
    Inject,
    Drop(DropReason),
}

impl From<DropReason> for StoryRefusalReason {
    fn from(reason: DropReason) -> Self {
        match reason {
            DropReason::Disabled => Self::Disabled,
            DropReason::WrongRole => Self::WrongRole,
            DropReason::NotInPlay => Self::NotInPlay,
            DropReason::WrongPhase => Self::WrongPhase,
            DropReason::NotAStoryCall => Self::NotAStoryCall,
            DropReason::Duplicate => Self::Duplicate,
        }
    }
}

impl StoryRelayLedger {
    pub fn on_fire(
        &mut self,
        argument_pointer: u64,
        facts: &FireFacts,
        key: CallKey,
        now_ms: u64,
    ) -> FireAction {
        if let Some(how) = self.take_injected(argument_pointer, key, now_ms) {
            return FireAction::RunInjected(how);
        }
        let relayed_kind = facts.kind.is_some_and(StoryCallKind::is_relayed);
        let can_relay = facts.enabled
            && facts.connected
            && facts.local_in_play
            && facts.partner_in_same_phase
            && facts.args_relayable
            && !facts.owned_elsewhere
            && relayed_kind;
        if !can_relay {
            return FireAction::RunLocally;
        }
        match facts.role {
            FollowRole::Host => {
                if self.take_accepted(key, now_ms) {
                    return FireAction::DropDuplicate;
                }
                self.note_ran_here(key, now_ms);
                FireAction::RunAndAnnounce
            }
            FollowRole::Guest => {
                if self.take_ran_there(key, now_ms) {
                    return FireAction::DropDuplicate;
                }
                self.hold_call(key, now_ms);
                FireAction::SuppressAndAsk
            }
            FollowRole::Unknown => FireAction::RunLocally,
        }
    }

    pub fn on_trigger(&mut self, facts: &RunFacts, key: CallKey, now_ms: u64) -> TriggerAction {
        if let Some(reason) = refusal(facts, FollowRole::Host) {
            return TriggerAction::Drop(reason);
        }
        if !self.accept_trigger(key, now_ms) {
            return TriggerAction::Drop(DropReason::Duplicate);
        }
        TriggerAction::InjectAndBroadcast
    }

    pub fn on_run(&mut self, facts: &RunFacts, key: CallKey, now_ms: u64) -> RunAction {
        self.resolve_held(key);
        if let Some(reason) = refusal(facts, FollowRole::Guest) {
            return RunAction::Drop(reason);
        }
        self.note_ran_there(key, now_ms);
        RunAction::Inject
    }
}

fn refusal(facts: &RunFacts, wanted_role: FollowRole) -> Option<DropReason> {
    if !facts.enabled {
        return Some(DropReason::Disabled);
    }
    if facts.role != wanted_role {
        return Some(DropReason::WrongRole);
    }
    if !facts.local_in_play {
        return Some(DropReason::NotInPlay);
    }
    if facts.local_phase != Some(facts.call_phase) {
        return Some(DropReason::WrongPhase);
    }
    if !facts.known_story_call {
        return Some(DropReason::NotAStoryCall);
    }
    None
}
